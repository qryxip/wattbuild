import itertools
import json
import os
import shutil
import subprocess
import textwrap
from argparse import ArgumentParser
from pathlib import Path
from subprocess import PIPE
from tempfile import TemporaryDirectory


def main() -> None:
    parser = ArgumentParser()
    parser.add_argument('--prefer-sccache', action='store_true')
    parser.add_argument('--proc-macro2-rev', nargs='?', default=None)
    parser.add_argument('build_dependencies', nargs='+')
    args = parser.parse_args()

    with TemporaryDirectory(prefix='wattbuild-') as tempdir:
        if args.proc_macro2_rev is None:
            rev = ''
        else:
            rev = f', rev = "{args.proc_macro2_rev}"'

        manifest = textwrap.dedent(
            f'''\
            [patch.crates-io]
            proc-macro2 = {{ git = "https://github.com/dtolnay/watt"{rev} }}

            [package]
            name = "wattbuild-build"
            version = "0.0.0"
            edition = "2018"

            [build-dependencies]
            '''
        )

        for i, value in enumerate(args.build_dependencies):
            manifest += f'_{i} = {value}\n'

        with open(Path(tempdir, 'Cargo.toml'), 'w') as file:
            file.write(manifest)
        Path(tempdir, 'src').mkdir()
        with open(Path(tempdir, 'src', 'lib.rs'), 'w') as file:
            file.write('')

        metadata = json.loads(subprocess.run(
            [os.environ['CARGO'], 'metadata', '--format-version', '1'],
            stdout=PIPE, cwd=tempdir, check=True,
        ).stdout.decode())

        node = next(node for node in metadata['resolve']['nodes']
                    if node['id'] == metadata['resolve']['root'])

        build_dependencies = [package for package in metadata['packages']
                              if package['id'] in node['dependencies']]

        env = os.environ.copy()
        if args.prefer_sccache:
            sccache_exe = shutil.which('sccache')
            if sccache_exe is not None:
                env['RUSTC_WRAPPER'] = sccache_exe

        subprocess.run(
            [os.environ['CARGO'], 'build', '--release',
             *itertools.chain.from_iterable(
                 ['-p', f'{package["name"]}:{package["version"]}']
                 for package in build_dependencies
             ),
             '--target', 'wasm32-unknown-unknown'],
            stdout=PIPE, cwd=tempdir, env=env, check=True,
        )

        for package in build_dependencies:
            name = next(
                (target['name'] for target in package['targets']
                 if 'cdylib' in target['kind']),
                None,
            )
            if name is None:
                raise Exception(f'no `cdylib` target in `{package["id"]}`')
            path = Path(metadata['target_directory'], 'wasm32-unknown-unknown',
                        'release', f'{name.replace("-", "_")}.wasm')
            shutil.copy(path, os.environ['OUT_DIR'])


if __name__ == '__main__':
    main()
