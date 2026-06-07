import json

output_path = 'C:/Users/18811/AppData/Local/Temp/claude/E--3370-Desktop-XFast-Manager/5cfd7904-80a6-41e0-bc8f-85ec98e0040a/tasks/wx6ubky1n.output'
data = json.load(open(output_path, encoding='utf-8'))
translations = data['result']['translations']

for t in translations:
    code = t['code']
    text = t['text'].strip()
    # Strip markdown fences
    if text.startswith('```'):
        text = text.split('\n', 1)[1] if '\n' in text else text
    text = text.replace('```', '').strip()

    # Ensure proper indentation
    lines = text.split('\n')
    indented = []
    for line in lines:
        if line.strip():
            if not line.startswith('  '):
                indented.append('  ' + line)
            else:
                indented.append(line)
        else:
            indented.append(line)
    text = '\n'.join(indented)

    # Read the locale file
    locale_path = 'E:/3370/Desktop/XFast-Manager/src/i18n/' + code + '.ts'
    with open(locale_path, 'r', encoding='utf-8') as f:
        content = f.read()

    # Find "} satisfies LocaleSchema" at the end and insert doctor before it
    if '} satisfies LocaleSchema' in content:
        parts = content.rsplit('} satisfies LocaleSchema', 1)
        # Insert before the final "} satisfies"
        # The structure is:  ...altitude closing "  },\n} satisfies..."
        # We want: ...altitude closing "  },\n  doctor...,\n} satisfies..."
        new_content = parts[0].rstrip() + '\n  ' + text + ',\n} satisfies LocaleSchema' + parts[1]
        with open(locale_path, 'w', encoding='utf-8', newline='\n') as f:
            f.write(new_content)
        print('OK ' + code)
    else:
        print('ERROR ' + code + ': satisfies LocaleSchema not found')

print('All translations injected.')
