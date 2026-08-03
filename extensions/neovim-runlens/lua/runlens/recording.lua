local Recording = {}
local active = false

function Recording.toggle(daemon)
  if active then
    daemon:call('record.stop', {}, function(err)
      if err then
        vim.notify('[runlens] stop recording: ' .. err, vim.log.levels.ERROR)
        return
      end
      active = false
      vim.notify('[runlens] recording stopped')
    end)
  else
    daemon:call('record.start', {}, function(err, result)
      if err then
        vim.notify('[runlens] start recording: ' .. err, vim.log.levels.ERROR)
        return
      end
      active = true
      local sid = (result.session_id or ''):sub(1, 8)
      vim.notify('[runlens] recording session ' .. sid)
    end)
  end
end

function Recording.status()
  return active
end

return Recording