local bit = require('bit')
local Daemon = {}
Daemon.__index = Daemon

function Daemon.new(opts)
  opts = opts or {}
  return setmetatable({
    host = opts.host or 'localhost',
    port = opts.port or 9876,
    ch = nil,
    connected = false,
    state = 'idle',
    buf = '',
    pending = {},
    next_id = 1,
    subs = {},
    on_connect = nil,
  }, Daemon)
end

function Daemon:_ws_key()
  local b = {}
  for i = 1, 16 do b[i] = math.random(0, 255) end
  return vim.base64.encode(string.char(table.unpack(b)))
end

function Daemon:connect(callback)
  self.on_connect = callback
  local key = self:_ws_key()
  local ok, ch = pcall(vim.fn.sockconnect, 'tcp', self.host, self.port, {
    on_data = function(_, data) self:_on_data(data) end,
  })
  if not ok or ch == 0 then
    self:_error('connect', 'could not connect to ' .. self.host .. ':' .. self.port)
    return
  end
  self.ch = ch
  local req = 'GET / HTTP/1.1\r\nHost: ' .. self.host .. ':' .. self.port
    .. '\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n'
    .. 'Sec-WebSocket-Key: ' .. key .. '\r\nSec-WebSocket-Version: 13\r\n\r\n'
  vim.fn.chansend(ch, req)
  self.state = 'connecting'
end

function Daemon:_on_data(data)
  self.buf = self.buf .. (data or '')
  if self.state == 'connecting' then
    local pos = self.buf:find('\r\n\r\n')
    if not pos then return end
    local header = self.buf:sub(1, pos - 1)
    self.buf = self.buf:sub(pos + 4)
    self.state = 'open'
    if not header:find('101') then
      self:_error('connect', 'WebSocket upgrade failed')
      return
    end
    self.connected = true
    vim.schedule(function()
      if self.on_connect then self.on_connect(nil, true) end
    end)
  end
  if self.state == 'open' and #self.buf > 0 then
    self:_process_frames()
  end
end

function Daemon:_process_frames()
  while #self.buf >= 2 do
    local b0 = string.byte(self.buf, 1)
    local b1 = string.byte(self.buf, 2)
    local opcode = bit.band(b0, 0x0F)
    local masked = bit.band(b1, 0x80) ~= 0
    local len = bit.band(b1, 0x7F)
    local off = 2
    if len == 126 then
      if #self.buf < off + 2 then break end
      len = string.byte(self.buf, off + 1) * 256 + string.byte(self.buf, off + 2)
      off = off + 2
    elseif len == 127 then
      if #self.buf < off + 8 then break end
      len = 0
      for i = 0, 7 do
        len = len * 256 + string.byte(self.buf, off + i + 1)
      end
      off = off + 8
    end
    if masked then
      if #self.buf < off + 4 + len then break end
      local mask = {}
      for i = 0, 3 do mask[i + 1] = string.byte(self.buf, off + i + 1) end
      off = off + 4
      local decoded = {}
      for i = 1, len do
        decoded[i] = string.char(bit.bxor(string.byte(self.buf, off + i), mask[(i - 1) % 4 + 1]))
      end
      self.buf = self.buf:sub(off + len)
      if opcode == 1 then
        self:_on_message(table.concat(decoded))
      end
    else
      if #self.buf < off + len then break end
      local payload = self.buf:sub(off, off + len - 1)
      self.buf = self.buf:sub(off + len)
      if opcode == 1 then
        self:_on_message(payload)
      end
    end
  end
end

function Daemon:_on_message(raw)
  local ok, msg = pcall(vim.json.decode, raw)
  if not ok then return end
  if msg.id then
    local pending = self.pending[msg.id]
    if pending then
      self.pending[msg.id] = nil
      if msg.error then
        pending(nil, msg.error)
      else
        pending(msg.result)
      end
    end
  elseif msg.method then
    local handler = self.subs[msg.method]
    if handler then handler(msg.params) end
  end
end

function Daemon:call(method, params, callback)
  local id = self.next_id
  self.next_id = id + 1
  self.pending[id] = callback
  local req = vim.json.encode({
    jsonrpc = '2.0',
    id = id,
    method = method,
    params = params or {},
  })
  vim.fn.chansend(self.ch, { req .. '\r\n' })
end

function Daemon:subscribe(method, handler)
  self.subs[method] = handler
end

function Daemon:_error(context, msg)
  self.connected = false
  self.state = 'error'
  vim.notify('[runlens] ' .. context .. ': ' .. msg, vim.log.levels.ERROR)
  vim.schedule(function()
    if self.on_connect then self.on_connect(msg) end
  end)
end

function Daemon:disconnect()
  if self.ch then
    vim.fn.chanclose(self.ch)
    self.ch = nil
  end
  self.connected = false
  self.state = 'idle'
  self.buf = ''
  self.pending = {}
  self.subs = {}
end

return Daemon