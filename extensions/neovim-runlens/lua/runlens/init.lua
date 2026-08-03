local Daemon = require('runlens.daemon')
local Session = require('runlens.session')
local Recording = require('runlens.recording')
local Graph = require('runlens.graph')

local daemon = nil

function setup(user_opts)
  user_opts = user_opts or {}
  daemon = Daemon.new({
    host = user_opts.host or 'localhost',
    port = user_opts.port or 9876,
  })

  daemon:connect(function(err)
    if err then
      vim.notify('[runlens] daemon unavailable - start with "runlens daemon"', vim.log.levels.WARN)
      return
    end
    vim.notify('[runlens] connected to daemon')
  end)
end

local function check_daemon()
  if not daemon then
    vim.notify('[runlens] not initialized - call require("runlens").setup()', vim.log.levels.ERROR)
    return false
  end
  if not daemon.connected then
    vim.notify('[runlens] daemon not connected', vim.log.levels.ERROR)
    return false
  end
  return true
end

function commands()
  vim.api.nvim_create_user_command('RunLensStatus', function()
    if not daemon then
      vim.notify('[runlens] not initialized', vim.log.levels.ERROR)
      return
    end
    local s = daemon.connected and 'connected' or 'disconnected'
    vim.notify('[runlens] daemon: ' .. s .. ' (' .. daemon.host .. ':' .. daemon.port .. ')')
  end, {})

  vim.api.nvim_create_user_command('RunLensList', function()
    if not check_daemon() then return end
    Session.list(daemon, function(err, sessions)
      if err then return end
      if #sessions == 0 then
        vim.notify('[runlens] no sessions')
        return
      end
      local items = {}
      for _, s in ipairs(sessions) do
        table.insert(items, {
          label = s.id:sub(1, 8) .. '  ' .. (s.event_count or 0) .. ' events  ' .. (s.duration_ms or 0) .. 'ms',
          session = s,
        })
      end
      vim.ui.select(items, {
        prompt = 'RunLens sessions:',
        format_item = function(item) return item.label end,
      }, function(choice)
        if choice then
          pcall(vim.ui.open, 'runlens://session/' .. choice.session.id)
        end
      end)
    end)
  end, {})

  vim.api.nvim_create_user_command('RunLensRecord', function()
    if not check_daemon() then return end
    Recording.toggle(daemon)
  end, {})

  vim.api.nvim_create_user_command('RunLensGraph', function()
    if not check_daemon() then return end
    Session.list(daemon, function(err, sessions)
      if err then return end
      if #sessions == 0 then
        vim.notify('[runlens] no sessions')
        return
      end
      local items = {}
      for _, s in ipairs(sessions) do
        table.insert(items, {
          label = s.id:sub(1, 8) .. '  ' .. (s.event_count or 0) .. ' events',
          session = s,
        })
      end
      vim.ui.select(items, {
        prompt = 'Select session for critical path:',
        format_item = function(item) return item.label end,
      }, function(choice)
        if choice then Graph.show_critical(daemon, choice.session.id) end
      end)
    end)
  end, {})
end

return { setup = setup, commands = commands }