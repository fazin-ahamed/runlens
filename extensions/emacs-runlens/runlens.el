;;; runlens.el --- RunLens developer flight recorder integration  -*- lexical-binding: t; -*-
;;
;; Copyright (c) 2026 RunLens authors
;; SPDX-License-Identifier: MIT
;;
;; Quick start:
;;   M-x runlens-mode           Enable minor mode (global keybindings)
;;   M-x runlens-list           List recorded sessions
;;   M-x runlens-record         Toggle recording
;;   M-x runlens-critical-path  Show critical path for a session
;;
;;; Code:

(require 'json)
(require 'tabulated-list)
(require 'transient)
(eval-when-compile
  (require 'cl-lib))

(defgroup runlens nil
  "RunLens daemon integration."
  :group 'tools)

(defcustom runlens-daemon-host "127.0.0.1"
  "RunLens daemon host."
  :type 'string :group 'runlens)

(defcustom runlens-daemon-port 9876
  "RunLens daemon port."
  :type 'integer :group 'runlens)

(defcustom runlens-binary "runlens"
  "Path to the runlens CLI binary."
  :type 'file :group 'runlens)

(defvar runlens--ws nil
  "WebSocket connection handle (websocket.el).")

(defvar runlens--recording-p nil
  "Non-nil when a recording session is active.")

(defvar runlens--request-id 0
  "Monotonic JSON-RPC request ID.")

;;; Daemon client

(defun runlens--ws-url ()
  (format "ws://%s:%d" runlens-daemon-host runlens-daemon-port))

(defun runlens-daemon-connected-p ()
  "Return t if the WebSocket connection is live."
  (and (featurep 'websocket)
       runlens--ws
       (eq (websocket-ready-state runlens--ws) 'open)))

(defun runlens-connect ()
  "Connect to the RunLens daemon via WebSocket.
Falls back to `runlens-binary' when `websocket.el' is missing."
  (unless (featurep 'websocket)
    (message "RunLens: websocket.el not available, using CLI fallback")
    (cl-return-from runlens-connect))
  (when (runlens-daemon-connected-p)
    (message "RunLens: already connected")
    (cl-return-from runlens-connect))
  (condition-case err
      (progn
        (require 'websocket)
        (setq runlens--ws
              (websocket-open
               (runlens--ws-url)
               :on-message (lambda (_ws frame)
                             (runlens--on-message (websocket-frame-payload frame)))
               :on-close (lambda (_ws) (setq runlens--ws nil))
               :on-error (lambda (_ws err) (message "RunLens WS error: %s" err))))
        (message "RunLens: connected to %s" (runlens--ws-url)))
    (error (message "RunLens: connection failed (%s), using CLI fallback" err))))

(defun runlens-disconnect ()
  "Disconnect from the daemon."
  (when (runlens-daemon-connected-p)
    (websocket-close runlens--ws))
  (setq runlens--ws nil)
  (message "RunLens: disconnected"))

(defun runlens--rpc-call (method params)
  "Send JSON-RPC request and return decoded result.
If WebSocket is not connected, fall back to `runlens' CLI."
  (if (runlens-daemon-connected-p)
      (let* ((id (cl-incf runlens--request-id))
             (request (json-encode `((jsonrpc . "2.0")
                                     (id . ,id)
                                     (method . ,method)
                                     (params . ,params))))
             (result-var nil)
             (condition (make-condition-variable (make-mutex) "runlens-rpc")))
        (let ((sync-callback (lambda (result)
                               (setq result-var result)
                               (condition-notify condition))))
          (puthash id sync-callback (make-hash-table :test 'eql))
          (websocket-send-text runlens--ws request)
          (condition-wait condition)
          result-var))
    (runlens--cli-call method params)))

(defun runlens--cli-call (method params)
  "Call the `runlens' CLI binary directly."
  (let* ((args (pcase method
                 ("record.start" (list "record" "--start" "--json"))
                 ("record.stop" (list "record" "--stop" "--json"))
                 ("session.list" (list "list" "--limit"
                                       (number-to-string (or (plist-get params :limit) 50))
                                       "--json"))
                 ("session.get" (list "get" (plist-get params :id) "--json"))
                 ("graph.critical" (list "graph" "critical"
                                         (plist-get params :trace_id) "--json"))
                 ("daemon.status" (list "daemon" "status"))
                 (_ (error "RunLens: unknown method %s" method))))
         (output (shell-command-to-string
                  (mapconcat #'shell-quote-argument (cons runlens-binary args) " "))))
    (when (and output (not (string-empty-p (string-trim output)))
               (not (string-match-p "^Error" output)))
      (json-read-from-string output))))

(defun runlens--on-message (payload)
  "Process a JSON-RPC response frame."
  (condition-case nil
      (let* ((msg (json-read-from-string payload))
             (msg-id (cdr (assq 'id msg)))
             (sync-callback (and msg-id (gethash msg-id (make-hash-table :test 'eql)))))
        (if sync-callback
            (funcall sync-callback (cdr (assq 'result msg)))
          (message "RunLens: received notification %s" (cdr (assq 'method msg)))))
    (error (message "RunLens: failed to parse message: %s" payload))))

;;; Interactive commands

(defun runlens-list ()
  "List recorded sessions."
  (interactive)
  (let* ((result (runlens--rpc-call "session.list" '(:limit 50)))
         (sessions (if (vectorp result) result [])))
    (if (= (length sessions) 0)
        (message "RunLens: no sessions")
      (with-current-buffer (get-buffer-create "*RunLens Sessions*")
        (setq tabulated-list-format [("ID" 10 t) ("Events" 8 t) ("Label" 30 t)])
        (setq tabulated-list-entries
              (mapcar (lambda (s)
                        (let ((id (or (cdr (assq 'id s)) ""))
                              (ev (or (cdr (assq 'event_count s)) 0))
                              (label (or (cdr (assq 'label s)) "")))
                          (list id (vector (substring id 0 (min 8 (length id)))
                                           (number-to-string ev) label))))
                      (append sessions nil)))
        (tabulated-list-print)
        (display-buffer (current-buffer))))))

(defun runlens-record ()
  "Toggle recording."
  (interactive)
  (let ((result (if runlens--recording-p
                    (runlens--rpc-call "record.stop" '(:jsonrpc "2.0"))
                  (runlens--rpc-call "record.start" '(:jsonrpc "2.0")))))
    (setq runlens--recording-p (not runlens--recording-p))
    (message "RunLens: recording %s"
             (if runlens--recording-p "started" "stopped"))))

(defun runlens-critical-path (session-id)
  "Show critical path for a session."
  (interactive "sSession ID: ")
  (let ((result (runlens--rpc-call "graph.critical"
                                   (list :trace_id session-id))))
    (if result
        (message "RunLens critical path: %s" result)
      (message "RunLens: no critical path data"))))

;;; Minor mode

(defvar runlens-mode-map
  (let ((map (make-sparse-keymap)))
    (define-key map (kbd "C-c r l") 'runlens-list)
    (define-key map (kbd "C-c r r") 'runlens-record)
    (define-key map (kbd "C-c r c") 'runlens-critical-path)
    map)
  "Keymap for RunLens mode.")

(define-minor-mode runlens-mode
  "Toggle RunLens integration minor mode."
  :lighter " RL"
  :keymap runlens-mode-map
  (if runlens-mode
      (runlens-connect)
    (runlens-disconnect)))

(provide 'runlens)
;;; runlens.el ends here