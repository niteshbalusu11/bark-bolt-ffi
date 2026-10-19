#!/usr/bin/env python3
"""Require every upstream API name in the generated Swift and Kotlin contracts."""
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[1]
baseline = json.loads((root / 'docs/api-baseline.json').read_text())
swift = '\n'.join(p.read_text() for p in (root / 'dist/swift').glob('*.swift'))
kotlin = '\n'.join(p.read_text() for p in (root / 'dist/kotlin').rglob('*.kt'))
assert swift and kotlin, 'Generate both SDKs with --deny-skipped first'


def camel(name):
    first, *parts = name.split('_')
    return first + ''.join(p[0].upper() + p[1:] for p in parts)


for export in baseline['exports']:
    name = camel(export['name'])
    for language, source in [('Swift', swift), ('Kotlin', kotlin)]:
        # Scope to the generated object rather than accepting another method
        # with the same name. Constructors are static factories in these SDKs.
        owner = export['owner']
        scope = source
        if owner:
            start = re.search(r'(?m)^(?:public (?:final )?)?class ' + owner + r'\b', source)
            assert start, f'{language}: missing {owner}'
            end = re.search(r'\n(?:public (?:final )?)?class \w+', source[start.end():])
            scope = source[start.start():start.end() + end.start()] if end else source[start.start():]
        if language == 'Swift' and export['name'] == 'custom' and owner == 'OnchainWallet':
            assert 'init(custom ' in scope
        else:
            assert re.search(r'\b(?:func|fun) `?' + name + r'`?\s*\(', scope), f'{language}: missing {owner}.{name}'
for callback in baseline['callbacks']:
    for method in callback['methods']:
        for source in [swift, kotlin]:
            assert re.search(r'\b(?:func|fun) `?' + camel(method) + r'`?\s*\(', source), method
for kind in ['records', 'enums', 'callbacks']:
    for item in baseline[kind]:
        name = item['name']
        for language, source in [('Swift', swift), ('Kotlin', kotlin)]:
            assert re.search(r'\b(?:struct|enum|class|interface|protocol) ' + name + r'\b', source), f'{language}: missing {name}'
for source in [swift, kotlin]:
    assert re.search(r'\b(?:func|fun) signPsbt\s*\(', source), 'Missing master sign-only callback'
    assert re.search(r'\b(?:vtxoSwept|VtxoSwept)\b', source), 'Missing master swept exit state'
for record, fields in baseline.get('field_contracts', {}).items():
    for language, source in [('Swift', swift), ('Kotlin', kotlin)]:
        start = re.search(r'\b(?:struct|class) ' + record + r'\b', source)
        assert start, f'{language}: missing {record}'
        following = source[start.end():]
        end = re.search(r'\n(?:public )?(?:data )?(?:struct|class|enum|interface|protocol) \w+', following)
        scope = following[:end.start()] if end else following
        for field in fields:
            assert re.search(r'\b(?:let|var|val) ' + camel(field) + r'\s*:', scope), f'{language}: missing {record}.{camel(field)}'
print(f"API parity: {len(baseline['exports'])} exports, {len(baseline['records'])} records, {len(baseline['enums'])} enums, {len(baseline['callbacks'])} callbacks present in both hosts")
