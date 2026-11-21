#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Check that shared function instrumentation is specific to each harness."""

import collections
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

HERE = Path(__file__).resolve().parent
NAMES = ['delayed_harness', 'initialized_control_harness']
UNINIT = 'Undefined Behavior: Reading from an uninitialized pointer'


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def inventory(stdout):
    harnesses = {}
    current = None
    prop = None
    for line in stdout.splitlines():
        match = re.fullmatch(r'Checking harness (.+)\.\.\.', line)
        if match:
            current = match[1]
            require(current not in harnesses, 'duplicate harness')
            harnesses[current] = []
            prop = None
        match = re.fullmatch(r'Check (\d+): (.+)', line)
        if current and match:
            prop = {'index': int(match[1]), 'id': match[2]}
            harnesses[current].append(prop)
        elif prop:
            match = re.fullmatch(r'\s+- (Status|Description|Location): (.*)', line)
            if match:
                key, value = match.groups()
                if key == 'Description' and value.startswith('"') and value.endswith('"'):
                    value = value[1:-1]
                prop[key.lower()] = value
    return harnesses


def parse(doc, properties, wanted, code, uninit):
    require(doc['schema_version'] == '0.1.0' and doc['run_state'] == 'COMPLETE'
            and doc['outcome'] == {'kind': 'COMPLETED'}, 'incomplete export')
    features = {'export-json', 'ghost-state', 'uninit-checks'} if uninit else {'export-json'}
    require(set(doc['enabled_unstable_features']) == features
            and len(doc['enabled_unstable_features']) == len(features), 'check features')
    config = doc['configuration']
    require(config['coverage_enabled'] is False and config['cbmc_args'] == [], 'solver configuration')
    for key in ('memory_safety', 'overflow', 'unwinding', 'undefined_function', 'assertion_reach_checks'):
        require(config['checks'][key] is True, 'disabled check: ' + key)
    require(config['checks']['prove_safety_only'] is False
            and config['checks']['ignore_global_asm'] is False, 'check configuration')
    selection = doc['harness_selection']
    require(selection['exact'] is True and selection['requested_filters'] == wanted
            and selection['unmatched_filters'] == [] and selection['matched_count'] == len(wanted),
            'harness selection')
    entries = doc['harnesses']
    require(len(entries) == len(wanted) and {h['name'] for h in entries} == set(wanted)
            and set(properties) == set(wanted), 'harness inventory')
    result = {}
    for entry in entries:
        name = entry['name']
        props = properties[name]
        checks = entry['checks']
        require(props and len(props) == entry['n_properties'] == checks['total'], 'partial inventory')
        require([p['index'] for p in props] == list(range(1, len(props) + 1)), 'check indices')
        require(len({p['id'] for p in props}) == len(props), 'duplicate check')
        for prop in props:
            require(all(isinstance(prop[k], str) and prop[k]
                        for k in ('id', 'status', 'description', 'location')), 'check identity')
            require(prop['status'] in ('SUCCESS', 'FAILURE', 'UNREACHABLE'), 'unresolved check')
        counts = collections.Counter(p['status'] for p in props)
        require(counts['SUCCESS'] == checks['success'], 'success count')
        for bucket, status in (('failure', 'FAILURE'), ('unreachable', 'UNREACHABLE')):
            require(sorted(checks[bucket]) == sorted(p['id'] for p in props if p['status'] == status),
                    'check partition')
        require(all(checks[k] == [] for k in ('undetermined', 'error', 'unknown', 'other')),
                'incomplete checks')
        require(entry['covers']['total'] == 0, 'unexpected coverage')
        verdict = entry['outcome']
        require(verdict == {'kind': 'COMPLETED',
                            'verdict': 'FAILURE' if counts['FAILURE'] else 'SUCCESS'}, 'verdict')
        failed = entry['failed_properties']
        require(entry['n_failed'] == len(failed) == counts['FAILURE'], 'failure count')
        require(sorted(p['id'] for p in failed) == sorted(checks['failure']), 'failure identities')
        for prop in failed + entry['unsupported_constructs']:
            native = next(p for p in props if p['id'] == prop['id'])
            require(all(prop[k] == native[k] for k in ('status', 'description')), 'property mismatch')
        require(all(p['status'] in ('SUCCESS', 'UNREACHABLE')
                    for p in entry['unsupported_constructs']), 'unsupported construct')
        result[name] = dict(entry, properties=props, verdict=verdict['verdict'])
    require(code == int(any(h['verdict'] == 'FAILURE' for h in result.values())), 'process exit')
    return result


def signature(harness):
    # Compare semantic check identities while preserving multiplicity and source locations.
    return sorted((re.sub(r'\.\d+$', '', p['id']), p['status'], p['description'], p['location'])
                  for p in harness['properties'])


def unsafe_control(harness, file, line):
    require(harness['verdict'] == 'FAILURE' and harness['failed_properties'], 'missing unsafe failure')
    for prop in harness['failed_properties']:
        require(prop['status'] == 'FAILURE' and prop['class'] == 'safety_check'
                and prop['description'].startswith(UNINIT) and prop['file'].endswith(file)
                and prop['line'] == line, 'unexpected unsafe failure')


def ordinary_control(harness):
    require(harness['verdict'] == 'SUCCESS' and harness['failed_properties'] == [],
            'ordinary checking failed')
    require(all(not p['description'].startswith(UNINIT) for p in harness['properties']),
            'unexpected uninitialized-memory check')


def compare(runs, uninit):
    independent = runs['uninitialized_control']['uninitialized_control']
    if uninit:
        unsafe_control(independent, 'control.rs', '16')
    else:
        ordinary_control(independent)
    for mode in ('delayed_first', 'control_first'):
        delayed = runs[mode + '/delayed_alone'][NAMES[0]]
        initialized = runs[mode + '/control_alone'][NAMES[1]]
        pair = runs[mode + '/paired']
        if uninit:
            unsafe_control(delayed, 'src/lib.rs', '12')
            unsafe_control(pair[NAMES[0]], 'src/lib.rs', '12')
        else:
            for harness in (delayed, initialized, *pair.values()):
                ordinary_control(harness)
        for harness in (initialized, pair[NAMES[1]]):
            require(harness['verdict'] == 'SUCCESS' and any(
                p['description'] == 'assertion failed: result == 7' and p['status'] == 'SUCCESS'
                for p in harness['properties']), 'initialized harness failed')
        for name, singleton in zip(NAMES, (delayed, initialized)):
            require(signature(singleton) == signature(pair[name]), 'shared function checks changed')


def probe(command, wanted, export, env, uninit):
    export.unlink(missing_ok=True)
    args = command + ['--exact', '-j1', '-Z', 'export-json',
                      '--export-json', str(export), '--output-format=regular']
    if uninit:
        args += ['-Z', 'ghost-state', '-Z', 'uninit-checks']
    for name in wanted:
        args += ['--harness', name]
    process = subprocess.run(args, cwd=HERE, env=env, capture_output=True, text=True, timeout=120)
    require(process.returncode in (0, 1), process.stderr)
    try:
        return parse(json.loads(export.read_text()), inventory(process.stdout), wanted, process.returncode, uninit)
    except (OSError, ValueError, KeyError, TypeError, StopIteration):
        print(process.stdout)
        print(process.stderr)
        raise


def main():
    env = os.environ.copy()
    for key in ('CARGO_ENCODED_RUSTFLAGS', 'CARGO_TARGET_DIR', 'CARGO_BUILD_TARGET'):
        env.pop(key, None)
    env.update(RUSTFLAGS='', KANIFLAGS='', KANI_LOG='', NO_COLOR='1')
    with tempfile.TemporaryDirectory(prefix='delayed-shared-body-') as temporary:
        scratch = Path(temporary)
        export = scratch / 'results.json'
        for uninit in (True, False):
            checks = 'uninit' if uninit else 'ordinary'
            runs = {'uninitialized_control': probe(
                ['kani', str(HERE / 'control.rs'), '--target-dir', str(scratch / (checks + '-control'))],
                ['uninitialized_control'], export, dict(env, RUSTFLAGS='--edition=2021'), uninit)}
            # The feature selects call paths through the shared function; compiler order is unspecified.
            for mode in ('delayed_first', 'control_first'):
                for case, wanted in (('delayed_alone', NAMES[:1]), ('control_alone', NAMES[1:]), ('paired', NAMES)):
                    command = ['cargo', 'kani', '--manifest-path', str(HERE / 'Cargo.toml'), '--lib',
                               '--target-dir', str(scratch / (checks + '-' + mode + '-' + case))]
                    if mode == 'delayed_first':
                        command += ['--features', 'delayed-first']
                    runs[mode + '/' + case] = probe(command, wanted, export, env, uninit)
            compare(runs, uninit)
    print('Shared function checks agree for singleton and paired harnesses.')


if __name__ == '__main__':
    main()
