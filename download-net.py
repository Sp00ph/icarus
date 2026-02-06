#!/usr/bin/env python3

import urllib.request
import hashlib


def main():
    name, hash = open("network.txt").read().strip().split()
    path = "./icarus.nnue"
    try:
        if hashlib.sha256(open(path, "rb").read()).digest().hex() == hash:
            print("Net already exists!")
            return
    except OSError:
        pass

    print(f"Downloading net {name} to {path}")
    net = urllib.request.urlopen(
        f"https://github.com/Sp00ph/icarus-nets/releases/download/{name}/{name}.nnue"
    ).read()
    if hashlib.sha256(net).digest().hex() != hash:
        print("Invalid hash!")
        exit(1)
    open(path, "wb").write(net)


if __name__ == "__main__":
    main()
