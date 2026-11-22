#!/bin/bash
# turn.sh <name> <cwd> <prompt> [qodercli args...]: one print-mode turn of the installed Qoder CLI, recorded byte for byte.
# Writes rec/<name>.ndjson, rec/<name>.stderr, rec/<name>.starts (the sentinels that started, if any) and a line in turns.log.
P=/var/tmp/probe-qoder; Q=${QODER:-$P/prefix/node_modules/.bin/qodercli}; R=$P/rec
name=$1; cwd=$2; prompt=$3; shift 3
: > $P/iso/starts.log
( cd "$cwd" && printf '%s' "$prompt" | timeout "${TURN_TIMEOUT:-200}" "$Q" -p --output-format stream-json --no-session-persistence -m "${MODEL:-Fast}" --reasoning-effort "${EFFORT:-low}" "$@" > $R/$name.ndjson 2> $R/$name.stderr ); rc=$?
[ -s $P/iso/starts.log ] && cp $P/iso/starts.log $R/$name.starts || rm -f $R/$name.starts
echo "$name exit=$rc model=${MODEL:-Fast} argv: -p --output-format stream-json --no-session-persistence -m ${MODEL:-Fast} --reasoning-effort ${EFFORT:-low} $* stdin: $prompt" >> $P/turns.log
python3 - "$R/$name.ndjson" "$R/$name.starts" "$name" "$rc" <<'PY'
import json,sys,os
f,starts,name,rc=sys.argv[1:5]
init=res=None; uses=[]; errs=0
for l in open(f,errors='replace'):
    try: m=json.loads(l)
    except Exception: continue
    if m.get('type')=='system' and m.get('subtype')=='init': init=m
    if m.get('type')=='result': res=m
    if m.get('type')=='assistant':
        for b in (m.get('message',{}).get('content') or []):
            if isinstance(b,dict) and b.get('type')=='tool_use': uses.append(b.get('name')+':'+json.dumps(b.get('input'))[:110])
    if m.get('type')=='user':
        for b in (m.get('message',{}).get('content') or []):
            if isinstance(b,dict) and b.get('type')=='tool_result' and b.get('is_error'): errs+=1
st=sorted(set(l.split()[0] for l in open(starts))) if os.path.exists(starts) else []
print(f"{name} exit={rc} v={init and init.get('qodercli_version')} model={init and init.get('model')} tools={init and init.get('tools')} mcp={[(s.get('name'),s.get('status')) for s in (init or {}).get('mcp_servers',[])]}")
print(f"   started={st} tool_uses={len(uses)} tool_errors={errs} denials={len((res or {}).get('permission_denials') or [])} subtype={res and res.get('subtype')} credits={res and res.get('total_credits')}")
for u in uses[:14]: print('     use',u)
print('   result:',str((res or {}).get('result'))[:600].replace('\n',' | '))
PY
