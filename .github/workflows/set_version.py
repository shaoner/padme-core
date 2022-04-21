#!/usr/bin/env python3

import toml
import sys

fname = sys.argv[1]
version = sys.argv[2]

cargo = toml.load(fname)
cargo['package']['version'] = version

f = open(fname,'w')
toml.dump(cargo, f)
f.close()
