local Graph = {}

function Graph.show_critical(daemon, session_id)
  daemon:call('graph.critical', { trace_id = session_id }, function(err, path)
    if err then
      vim.notify('[runlens] critical path: ' .. err, vim.log.levels.ERROR)
      return
    end
    if not path or #path == 0 then
      vim.notify('[runlens] no critical path data')
      return
    end
    local lines = { 'RunLens Critical Path:' }
    for _, n in ipairs(path) do
      local label = n.name or n.label or 'node'
      local dur = n.duration_ms and (' (' .. n.duration_ms .. 'ms)') or ''
      table.insert(lines, '  ' .. label .. dur)
    end
    vim.notify(table.concat(lines, '\n'))
  end)
end

function Graph.show_trace(daemon, session_id)
  daemon:call('graph.trace', { trace_id = session_id }, function(err, trace)
    if err then
      vim.notify('[runlens] trace: ' .. err, vim.log.levels.ERROR)
      return
    end
    if not trace then
      vim.notify('[runlens] no trace data')
      return
    end
    local nodes = trace.nodes or {}
    local edges = trace.edges or {}
    local lines = { 'RunLens Trace: ' .. #nodes .. ' nodes, ' .. #edges .. ' edges' }
    for _, n in ipairs(nodes) do
      local dur = n.duration_ms and (' (' .. n.duration_ms .. 'ms)') or ''
      table.insert(lines, '  [' .. (n.node_type or '?') .. '] ' .. (n.name or n.id or '?') .. dur)
    end
    vim.notify(table.concat(lines, '\n'))
  end)
end

return Graph