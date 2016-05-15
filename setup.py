from setuptools import setup, find_packages
from rust_ext import build_rust, install_with_rust, RustModule


rust_modules = [
    RustModule(
        'benedictine.core',
        'Cargo.toml',
    ),
]

setup(
    name='benedictine',
    version='0.0.1',
    author='cocuh',
    author_email='cocuh.kk@tsukuba.ac.jp',
    cmdclass={
        'build_rust': build_rust,
        'install_lib': install_with_rust,
    },
    options={
        'build_rust': {
            'modules': rust_modules,
        },
    },
    packages=find_packages(),
    install_requires=['gevent'],
    test_suite='nose.collector',
    tests_require=['nose'],
)
