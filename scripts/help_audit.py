#!/usr/bin/env python3
import json
import re
import subprocess
from pathlib import Path

ROOT = Path('/Users/thrill3r/projects-ict-engine/ict-engine')


def run_help(args):
    result = subprocess.run(
        ['cargo', 'run', '--quiet', '--', *args],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
    )
    return result.stdout


def command_list():
    text = run_help(['--help'])
    commands = []
    in_commands = False
    for line in text.splitlines():
        if line.strip() == 'Commands:':
            in_commands = True
            continue
        if not in_commands:
            continue
        if not line.strip():
            break
        m = re.match(r'^\s{2,}([a-z][a-z0-9-]+)\s+', line)
        if m and m.group(1) != 'help':
            commands.append(m.group(1))
    return commands


def parse_options(help_text):
    lines = help_text.splitlines()
    options = []
    in_options = False
    i = 0
    while i < len(lines):
        line = lines[i]
        if line.strip() == 'Options:':
            in_options = True
            i += 1
            continue
        if not in_options:
            i += 1
            continue
        if not line.strip():
            break
        if not re.match(r'^\s{2,}[-]', line):
            i += 1
            continue
        stripped = line.strip()
        if '  ' in stripped:
            left, right = re.split(r'\s{2,}', stripped, maxsplit=1)
            desc = right.strip()
            same_line = True
        else:
            left = stripped
            desc = ''
            same_line = False
            j = i + 1
            while j < len(lines):
                nxt = lines[j]
                if not nxt.strip():
                    break
                if re.match(r'^\s{2,}[-]', nxt):
                    break
                if re.match(r'^\s{10,}\S', nxt):
                    desc = (desc + ' ' + nxt.strip()).strip()
                    j += 1
                    continue
                break
            i = j - 1
        options.append(
            {
                'flag': left,
                'description': desc,
                'has_description': bool(desc),
                'same_line': same_line,
            }
        )
        i += 1
    return options


def main():
    commands = command_list()
    rows = []
    missing = []
    for cmd in commands:
        text = run_help([cmd, '--help'])
        opts = parse_options(text)
        cmd_missing = [o['flag'] for o in opts if o['flag'] != '-h, --help' and not o['has_description']]
        rows.append(
            {
                'command': cmd,
                'option_count': len(opts),
                'missing_description_count': len(cmd_missing),
                'missing_descriptions': cmd_missing,
            }
        )
        if cmd_missing:
            missing.append({'command': cmd, 'missing_descriptions': cmd_missing})

    summary = {
        'root_help_has_version_flag': '-V, --version' in run_help(['--help']),
        'command_count': len(commands),
        'commands_with_missing_help': len(missing),
        'status': 'pass' if not missing else 'needs_fix',
    }

    report = {
        'summary': summary,
        'commands': rows,
        'missing': missing,
    }
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
