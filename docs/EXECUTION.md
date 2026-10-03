# Execution contract

<!-- agent-kit:execution:begin -->
## Session-driven work

Read this before builds, test suites, executable proof/review loops,
uploads/downloads, migrations, training, batch campaigns, or background or
uncertain-duration work. On OpenCode, load `pty-session` once before launching;
reading its pointer is not loading it. The skill owns the detailed PTY procedure.
This contract does not install a plugin or grant new tool/goal permissions.

- Loading the skill does not require a PTY for every command. Keep known-short,
  bounded checks in foreground `shell`, including parallel quick tool calls and
  test/build commands already observed to finish in seconds. For substantial or
  uncertain-duration work and parallel long-running jobs, use
  `pty_spawn` when available, including through Code Mode. Finite sessions need
  `notifyOnExit=true` and explicit `timeoutSeconds` sized to expected runtime
  with reasonable headroom. Never use detached `nohup ... &`, shell `&`, or
  sleep-then-check loops to wait for completion.
  Choose the route from expected runtime and evidence, not the default shell
  timeout; increasing that timeout is not a substitute for notified execution.
- Launch independent jobs in the same turn, within safe resource limits; jobs
  sharing mutable outputs or needing another job's result are not independent.
  Do other independent work immediately. When none remains, end the turn;
  completion notifications, not idle tool calls, wake the agent. Ending the
  turn is safe and is the fastest path to your result: the completion notice
  is your next input, and an idle parent with pending notified work wakes
  automatically rather than stalling. Notices that arrive late or in a batch
  are normal — admission is not consumption, and a batch is the scheduler
  landing queued input at the next run, not a malfunction. A foreground sleep
  cannot receive the completion signal, so it can only make the wait longer;
  never sleep in place of an exit.
- Check that spawn output includes a `ParentSession` id. If it says `missing`,
  report broken notification routing and do not wait for `<pty_exited>` or poll.
  If PTY tools are absent, use only a host-supported notified background session
  with a finite timeout. If no completion mechanism exists, report the blocker.
- Treat `<pty_exited>` as completion evidence, not proof of task correctness.
  Verify required artifacts/assertions. `Timed Out: yes` or a suspected signal
  kill is failure even with exit code 0; inspect status/logs to diagnose it.
  `pty_read` fetches evidence or diagnoses, never asks whether a job is done.
- For multi-hour or hang-prone jobs, arm one alarm PTY beside the job:
  `/bin/sleep N`, `notifyOnExit=true`, `timeoutSeconds` greater than N. Choose N
  from expected pace/risk (minutes, not seconds). Its notification schedules one
  bounded health check of pace, logs, and resource pressure; it is not the job's
  completion notice. Then re-arm once or intervene. Never build a rapid alarm
  loop. Cancel the alarm when its job finishes; clean up finished sessions with
  `pty_kill(cleanup=true)` after collecting needed evidence.
- Long-lived servers/watchers/tunnels use PTY without exit notification or
  timeout unless requested. Bounded readiness checks against a live server are
  the only shell-sleep exception; never use them to poll job completion.
- Goals: when authorized by the user/host, pause an active goal before an idle
  session wait to avoid no-op auto-continuations; record that it was a wait pause.
  On hosts whose delivery wakes idle parents with pending notified work, ending
  the turn on such work is sufficient instead of pausing. Resume only that wait
  pause when authorized and useful work can proceed, never a user pause. An alarm wake alone does not complete the job or justify resuming
  the goal. If goal tools require explicit user permission, obtain it rather
  than override the tool contract. Do not close a goal while required jobs run.

The kit checker verifies this contract and its root pointer are installed;
it cannot prove runtime behavior. Report unavailable tools or notification
routing honestly. Do not claim a plugin is active from file presence.
<!-- agent-kit:execution:end -->
