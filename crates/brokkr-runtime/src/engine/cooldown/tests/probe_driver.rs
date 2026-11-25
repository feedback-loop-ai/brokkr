//! The fake driver's probe action, apart from `tests.rs` so that file stays
//! within ruling 4's 2,000-line test ceiling.

use super::Probed;

/// The first line of a probe that hangs: it ignores a closed pipe, so an
/// engine killed after the probe's markers but before its `accepted` lands
/// leaves the probe live, as a real hung driver is, rather than dead by
/// SIGPIPE and live only as a zombie until reaped (#1110).
const HANGS: &str = "trap '' PIPE";

/// What a probe's branch of the fake driver does.
pub(super) fn probe_action(
    probed: Probed,
    accepted: &str,
    checkpoint: &str,
    succeeded: &str,
    refused: &str,
) -> String {
    match probed {
        Probed::Answer => format!("{accepted}\n  {checkpoint}\n  {succeeded}"),
        Probed::Refuse => refused.to_string(),
        Probed::Hang => format!("{HANGS}\n  {accepted}\n  sleep 30"),
        Probed::HangThenAnswer => format!(
            "p=$(cat '@PROBECOUNT@' 2>/dev/null || echo 0)\n  p=$((p+1)); \
             printf '%s' \"$p\" > '@PROBECOUNT@'\n  if [ \"$p\" -eq 1 ]; then\n    \
             {HANGS}\n    {accepted}\n    sleep 30\n  else\n    {accepted}\n    {checkpoint}\n    \
             {succeeded}\n  fi"
        ),
        Probed::RefuseThenAnswer => format!(
            "p=$(cat '@PROBECOUNT@' 2>/dev/null || echo 0)\n  p=$((p+1)); \
             printf '%s' \"$p\" > '@PROBECOUNT@'\n  if [ \"$p\" -eq 1 ]; then\n    \
             {refused}\n  else\n    {accepted}\n    {checkpoint}\n    {succeeded}\n  fi"
        ),
        Probed::OtherRefusal => "printf '%s,\"type\":\"result\",\"effect_id\":\"%s\",\
             \"attempt_id\":\"%s\",\"status\":\"failed\",\"error\":\"provider refused\",\
             \"refusal\":{\"class\":\"other\",\"provider\":\"claude\",\"route\":\"first\",\
             \"words\":\"auth failed\"}}\\n' \"$base\" \"$eid\" \"$aid\""
            .to_string(),
        Probed::MalformedRefusal => "printf '%s,\"type\":\"result\",\"effect_id\":\"%s\",\
             \"attempt_id\":\"%s\",\"status\":\"failed\",\"error\":\"provider refused\",\
             \"refusal\":{\"class\":\"rate_limit\"}}\\n' \"$base\" \"$eid\" \"$aid\""
            .to_string(),
        Probed::Error => format!(
            "{accepted}\n  printf '%s,\"type\":\"result\",\"effect_id\":\"%s\",\
             \"attempt_id\":\"%s\",\"status\":\"failed\",\"error\":\"probe exploded\"}}\\n' \
             \"$base\" \"$eid\" \"$aid\""
        ),
        Probed::Unadvertised => succeeded.to_string(),
        Probed::Barrier => format!(
            "{accepted}\n  : > '@STARTED@'\n  i=0\n  while [ ! -e '@RELEASE@' ] && \
             [ $i -lt 600 ]; do sleep 0.05; i=$((i+1)); done\n  {checkpoint}\n  {succeeded}"
        ),
        Probed::AnswerThenLinger => {
            format!("{accepted}\n  {checkpoint}\n  {succeeded}\n  : > '@ANSWERED@'\n  sleep 30")
        }
        Probed::DescendantThenLinger => format!(
            "setsid sleep 30 < /dev/null > /dev/null 2>&1 &\n  printf '%s' \"$!\" > \
             '@DESCPID@'\n  {accepted}\n  {checkpoint}\n  {succeeded}\n  : > '@ANSWERED@'\n  \
             sleep 30"
        ),
        Probed::BarrierRefuse => format!(
            "{accepted}\n  : > '@STARTED@'\n  i=0\n  while [ ! -e '@RELEASE@' ] && \
             [ $i -lt 600 ]; do sleep 0.05; i=$((i+1)); done\n  {refused}"
        ),
    }
}
