from setuptools import setup, find_packages

setup(
    name='benedictine',
    version='0.0.1',
    author='cocuh',
    author_email='cocuh.kk@tsukuba.ac.jp',
    packages=find_packages(),
    install_requires=['gevent'],
    test_suite='nose.collector',
    tests_require=['nose'],
)