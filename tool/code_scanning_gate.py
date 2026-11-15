"""Decides whether open code-scanning alerts stop a release or the weekly run.

The alerts come from GitHub's code-scanning API, one JSON object per line
(`gh api --paginate .../code-scanning/alerts?state=open --jq '.[]'`),
from every tool and of every severity. Each open alert blocks, unless
code-scanning-ignores.toml lists its tool and rule with a reason and an
ignoreUntil date that has not passed. An entry without a reason or a
date, or with a date that has passed, blocks too; that is the review. An
entry whose alert is no longer open is listed so it can be removed.

    python3 tool/code_scanning_gate.py alerts.jsonl code-scanning-ignores.toml

Exit 0: nothing blocks. Exit 1: an open alert that is not accepted, or a
bad entry. Exit 2: the alerts or the ignore file cannot be read.
"""

import datetime
import json
import sys
import tomllib


def read_alerts(path):
    """The open alerts, each once even when several branches report it."""
    alerts = {}
    with open(path, encoding="utf-8") as f:
        for line in f:
            if line.strip():
                alert = json.loads(line)
                alerts[alert.get("html_url") or alert.get("number")] = alert
    return list(alerts.values())


def read_ignores(path):
    with open(path, "rb") as f:
        return tomllib.load(f).get("IgnoredAlerts") or []


def problems(entry, today):
    """Why an ignore entry cannot accept anything; empty when it can."""
    found = []
    if not str(entry.get("reason") or "").strip():
        found.append("no reason")
    until = entry.get("ignoreUntil")
    if not isinstance(until, datetime.date):
        found.append("no ignoreUntil date")
    elif until < today:
        found.append(f"ignoreUntil {until} has passed")
    return found


def key(alert):
    return ((alert.get("tool") or {}).get("name"), (alert.get("rule") or {}).get("id"))


def describe(alert):
    rule = alert.get("rule") or {}
    severity = rule.get("security_severity_level") or rule.get("severity") or "?"
    location = (alert.get("most_recent_instance") or {}).get("location") or {}
    tool, rule_id = key(alert)
    return f"{tool} {rule_id} ({severity}) {location.get('path', '?')} {alert.get('html_url', '')}"


def gate(alerts, ignores, today):
    """(lines to print, number of blocking items)."""
    lines, blocking = [], 0
    entries = {}
    for entry in ignores:
        entry_key = (entry.get("tool"), entry.get("rule"))
        bad = problems(entry, today)
        if bad:
            lines.append(f"BAD ENTRY: {entry_key[0]} {entry_key[1]}: {', '.join(bad)}")
            blocking += 1
        else:
            entries[entry_key] = entry
    for alert in alerts:
        entry = entries.get(key(alert))
        if entry:
            lines.append(f"accepted until {entry['ignoreUntil']}: {describe(alert)}")
        else:
            lines.append(f"OPEN: {describe(alert)}")
            blocking += 1
    seen = {key(alert) for alert in alerts}
    for entry_key in entries:
        if entry_key not in seen:
            lines.append(f"no open alert, the entry can go: {entry_key[0]} {entry_key[1]}")
    lines.append(f"{len(alerts)} open alerts, {blocking} blocking")
    return lines, blocking


def main(argv, today=None):
    if len(argv) != 3:
        print("usage: code_scanning_gate.py <alerts.jsonl> <ignores.toml>", file=sys.stderr)
        return 2
    try:
        alerts = read_alerts(argv[1])
        ignores = read_ignores(argv[2])
    except (OSError, ValueError) as error:
        print(f"cannot read the alerts or the ignores: {error}", file=sys.stderr)
        return 2
    lines, blocking = gate(alerts, ignores, today or datetime.date.today())
    for line in lines:
        print(line)
    return 1 if blocking else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
