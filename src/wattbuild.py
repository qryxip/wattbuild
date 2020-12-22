import itertools
import json
import logging
import os
import platform
import shutil
import subprocess
import textwrap
from argparse import ArgumentParser
from pathlib import Path
from subprocess import PIPE


def main() -> None:
    parser = ArgumentParser()
    parser.add_argument('--toolchain')
    parser.add_argument('--proc-macro2-rev', nargs='?', default=None)
    parser.add_argument('build_dependencies', nargs='+')
    args = parser.parse_args()

    logger = logging.getLogger()

    env = os.environ.copy()

    if args.toolchain:
        rustup_exe = shutil.which('rustup')
        if rustup_exe is None:
            raise Exception('`rustup` not found')

        env['CARGO'] = subprocess.run(
            [rustup_exe, 'which', 'cargo', '--toolchain', args.toolchain],
            stdout=PIPE, check=True,
        ).stdout.decode()

        cargo_command = [rustup_exe, 'run', args.toolchain, 'cargo']
    else:
        if Path(os.environ['CARGO']).stem != 'cargo':
            cargo_exe = str(Path(os.environ['CARGO']).with_stem('cargo'))
            if not Path(cargo_exe).exists():
                which_cargo = shutil.which('cargo')
                if which_cargo is None:
                    raise Exception('`cargo` not found')
                cargo_exe = which_cargo
            logger.warning(f'`{os.environ["CARGO"]}` → `{cargo_exe}`')
            env['CARGO'] = cargo_exe

        cargo_command = [env['CARGO']]

    workdir = cache_dir() / 'wattbuild'
    workdir.mkdir(parents=True, exist_ok=True)

    if args.proc_macro2_rev is None:
        rev = ''
    else:
        rev = f', rev = "{args.proc_macro2_rev}"'

    manifest = textwrap.dedent(
        f'''\
        [workspace]

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

    with open(workdir / 'Cargo.toml', 'w') as file:
        file.write(manifest)
    (workdir / 'src').mkdir(exist_ok=True)
    with open(workdir / 'src' / 'lib.rs', 'w') as file:
        file.write('')

    subprocess.run([*cargo_command, 'update'],
                   cwd=workdir, env=env, check=True)

    metadata = json.loads(subprocess.run(
        [*cargo_command, 'metadata', '--format-version', '1'],
        stdout=PIPE, cwd=workdir, env=env, check=True,
    ).stdout.decode())

    node = next(node for node in metadata['resolve']['nodes']
                if node['id'] == metadata['resolve']['root'])

    build_dependencies = [package for package in metadata['packages']
                          if package['id'] in node['dependencies']]

    subprocess.run(
        [*cargo_command, 'build', '--release',
         *itertools.chain.from_iterable(
             ['-p', f'{package["name"]}:{package["version"]}']
             for package in build_dependencies
         ),
         '--target', 'wasm32-unknown-unknown'],
        stdout=PIPE, cwd=workdir, env=env, check=True,
    )

    for path in Path(metadata['target_directory'], 'wasm32-unknown-unknown',
                     'release').glob('*.wasm'):
        shutil.copy(path, os.environ['OUT_DIR'])


def cache_dir() -> Path:
    system = platform.uname().system
    home = Path(os.path.expanduser('~'))

    if system == 'Windows':
        if 'APPDATA' in os.environ:
            return Path(os.environ['APPDATA'], 'Local')
        return home / 'AppData' / 'Local'

    if system == 'Darwin':
        return home / 'Library' / 'Caches'

    if 'XDG_CACHE_DIR' in os.environ:
        return Path(os.environ['XDG_CACHE_DIR'])
    return home / '.cache'


if __name__ == '__main__':
    main()
