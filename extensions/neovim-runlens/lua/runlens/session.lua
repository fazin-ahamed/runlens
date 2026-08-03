local Session = {}

function Session.list(daemon, callback)
  daemon:call('session.list', { limit = 50 }, function(err, sessions)
    if err then
      vim.notify('[runlens] list sessions: ' .. err, vim.log.levels.ERROR)
      callback(err)
      return
    end
    callback(nil, sessions or {})
  end)
end

function Session.get(daemon, id, callback)
  daemon:call('session.get', { id = id }, callback)
end

function Session.start(daemon, callback)
  daemon:call('session.start', {}, function(err, result)
    if err then
      vim.notify('[runlens] start session: ' .. err, vim.log.levels.ERROR)
      callback(err)
      return
    end
    vim.notify('[runlens] session started: ' .. (result.id or ''):sub(1, 8))
    callback(nil, result)
  end)
end

function Session.stop(daemon, callback)
  daemon:call('session.stop', {}, function(err, result)
    if err then
      vim.notify('[runlens] stop session: ' .. err, vim.log.levels.ERROR)
      callback(err)
      return
    end
    vim.notify('[runlens] session stopped')
    callback(nil, result)
  end)
end

return Session