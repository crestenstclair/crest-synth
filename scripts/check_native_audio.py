#!/usr/bin/env python3
"""Exercise the same native archives as Cargo, with C++ allocation counters."""
from pathlib import Path
import argparse, subprocess, sys
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--release', action='store_true', help='Check the optimized release archives.')
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
profile = 'release' if args.release else 'debug'
archives = list((root / 'target' / profile / 'build').glob('crest-synth-*/out/libcrest_audio.a'))
if not archives:
    sys.exit(f'Run cargo test {"--release " if args.release else ""}--lib upstream_audio before this witness.')
output = max(archives, key=lambda p: p.stat().st_mtime).parent
binary = output / 'native-audio-witness'
command = ['c++', '-std=c++20', '-O1', '-g', '-I'+str(output), str(root/'native/audio/tests/realtime.cpp'), '-o', str(binary)]
if sys.platform == 'darwin':
    interposer = output / 'libcrest_witness_interpose.dylib'
    subprocess.run(['c++', '-std=c++20', '-O1', '-dynamiclib', '-mmacosx-version-min=11.0',
                    str(root/'native/audio/tests/interpose.cpp'), '-o', str(interposer),
                    '-Wl,-install_name,@rpath/'+interposer.name], check=True)
    command += ['-mmacosx-version-min=11.0', str(interposer), '-Wl,-rpath,'+str(output)]
for name in ['audio','mutable','daisy','stk','msfa','nam','convolution','sfizz']:
    command.append(str(output/f'libcrest_{name}.a'))
command += ['-pthread']
subprocess.run(command, check=True)
subprocess.run([str(binary)], check=True)
