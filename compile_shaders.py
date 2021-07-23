#!/usr/bin/env python3

import glob
import subprocess
import os
import argparse


def get_mtime(fpath) -> float:
    try:
        stat = os.stat(fpath)
        return stat.st_mtime
    except FileNotFoundError:
        return 0.0


def main():
    base_src_path = 'shaders/src/'
    base_dst_path = 'shaders/spv/'
    for file in glob.glob('{}/**'.format(base_src_path), recursive=True):
        if not (file.endswith('.vert') or file.endswith('.frag')):
            continue

        dst_file = '{}.spv'.format(os.path.join(base_dst_path, file.replace(base_src_path, '')))
        os.makedirs(os.path.dirname(dst_file), exist_ok=True)

        if get_mtime(dst_file) > get_mtime(file):
            continue

        print('Process {}'.format(file))
        ret = subprocess.run(['glslc', '-o', dst_file, file])
        ret.check_returncode()


if __name__ == '__main__':
    main()
