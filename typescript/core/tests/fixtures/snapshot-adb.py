#!/usr/bin/env python3
"""Deterministic ADB fixture for the opt-in mobile snapshot transport test."""
import os
import sys
from pathlib import Path
args = sys.argv[1:]
if args == ['devices', '-l']:
    print('List of devices attached\nemulator-5554 device model:SnapshotFixture')
elif args[:2] == ['-s', 'emulator-5554']:
    command = args[2:]
    if command[:3] == ['shell', 'dumpsys', 'window']:
        print('mCurrentFocus=Window{42 u0 test.app/.Main}')
    elif command[:2] == ['shell', 'cat']:
        print(Path(os.environ['ALLWRIGHT_TEST_XML']).read_text())
    elif command[:3] == ['shell', 'uiautomator', 'dump']:
        print('UI hierarchy dumped')
    elif command[:3] == ['shell', 'input', 'tap']:
        with open(os.environ['ALLWRIGHT_TEST_TAPS'], 'a') as output:
            output.write(' '.join(command[3:]) + '\n')
    else:
        sys.exit('Unexpected fixture command: ' + repr(args))
else:
    sys.exit('Unexpected fixture command: ' + repr(args))
