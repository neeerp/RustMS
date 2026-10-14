#!/usr/bin/env python3
"""Copy subtrees from one NX (PKG4) file into another.

usage: nx_graft.py BASE DONOR OUT PATH[=DONOR_PATH]...

OUT is BASE with the children of each PATH replaced by the children of
DONOR_PATH in DONOR (DONOR_PATH defaults to PATH). PATH must exist in BASE;
the replaced node keeps its name. The donor's strings, bitmaps, and audio are
appended, and BASE's tables are rewritten after them, so the old tables become
unused space. The output is deterministic for the same inputs.
"""
import shutil, struct, sys

HEADER = "<4sIQIQIQIQ"
CHUNK = 1 << 24


class Nx:
    def __init__(self, path):
        self.f = open(path, "rb")
        (magic, self.nc, self.no, self.sc, self.so,
         self.bc, self.bo, self.ac, self.ao) = struct.unpack(HEADER, self.f.read(52))
        if magic != b"PKG4":
            raise ValueError("%s is not an NX file" % path)

    def read(self, off, n):
        self.f.seek(off)
        return self.f.read(n)

    def node(self, i):
        return self.read(self.no + i * 20, 20)

    def offset(self, table, i):
        return struct.unpack("<Q", self.read(table + i * 8, 8))[0]

    def string(self, i):
        off = self.offset(self.so, i)
        n = struct.unpack("<H", self.read(off, 2))[0]
        return self.read(off + 2, n)

    def children(self, i):
        _, child, count, _ = struct.unpack("<IIHH", self.node(i)[:12])
        return range(child, child + count)

    def resolve(self, path):
        i = 0
        for part in [p for p in path.split("/") if p]:
            for c in self.children(i):
                if self.string(struct.unpack("<I", self.node(c)[:4])[0]).decode() == part:
                    i = c
                    break
            else:
                raise KeyError(path)
        return i


def copy_range(src, dst, off, n):
    src.f.seek(off)
    while n:
        part = src.f.read(min(n, CHUNK))
        dst.write(part)
        n -= len(part)


def pad(out, align):
    out.write(b"\0" * (-out.tell() % align))


def graft(base_path, donor_path, out_path, specs):
    base, donor = Nx(base_path), Nx(donor_path)
    targets = []
    for spec in specs:
        path, _, dpath = spec.partition("=")
        targets.append((base.resolve(path), donor.resolve(dpath or path)))

    # Lay the donor nodes out breadth first, so each node's children are
    # contiguous, numbering them after the base nodes.
    new_nodes = []  # donor index of each appended node
    first_child = {}  # donor index -> new index of its first child
    queue = [d for _, d in targets]
    while queue:
        d = queue.pop(0)
        if d in first_child:
            continue
        kids = list(donor.children(d))
        first_child[d] = base.nc + len(new_nodes)
        new_nodes.extend(kids)
        queue.extend(kids)

    strings, bitmaps, audio = {}, {}, {}

    def remap(table, key, start):
        if key not in table:
            table[key] = start + len(table)
        return table[key]

    def record(d, name=None):
        raw = donor.node(d)
        dname, _, count, typ = struct.unpack("<IIHH", raw[:12])
        data = raw[12:]
        name = remap(strings, dname, base.sc) if name is None else name
        if typ == 3:
            data = struct.pack("<I", remap(strings, struct.unpack("<I", data[:4])[0], base.sc)) + data[4:]
        elif typ == 5:
            data = struct.pack("<I", remap(bitmaps, struct.unpack("<I", data[:4])[0], base.bc)) + data[4:]
        elif typ == 6:
            data = struct.pack("<I", remap(audio, struct.unpack("<I", data[:4])[0], base.ac)) + data[4:]
        child = first_child.get(d, 0) if count else 0
        return struct.pack("<IIHH", name, child, count, typ) + data

    appended = [record(d) for d in new_nodes]
    patched = {}
    for b, d in targets:
        name = struct.unpack("<I", base.node(b)[:4])[0]
        patched[b] = record(d, name)

    with open(out_path, "wb") as out:
        shutil.copyfileobj(open(base_path, "rb"), out, CHUNK)

        string_offsets = []
        for d in sorted(strings, key=strings.get):
            pad(out, 2)
            string_offsets.append(out.tell())
            s = donor.string(d)
            out.write(struct.pack("<H", len(s)) + s)

        bitmap_offsets = []
        for d in sorted(bitmaps, key=bitmaps.get):
            pad(out, 8)
            bitmap_offsets.append(out.tell())
            off = donor.offset(donor.bo, d)
            n = struct.unpack("<I", donor.read(off, 4))[0]
            copy_range(donor, out, off, n + 4)

        # An audio node stores its length, so look it up from any node using it.
        audio_length = {}
        for d in new_nodes:
            raw = donor.node(d)
            if struct.unpack("<H", raw[10:12])[0] == 6:
                audio_length[struct.unpack("<I", raw[12:16])[0]] = struct.unpack("<I", raw[16:20])[0]
        audio_offsets = []
        for d in sorted(audio, key=audio.get):
            pad(out, 8)
            audio_offsets.append(out.tell())
            copy_range(donor, out, donor.offset(donor.ao, d), audio_length[d])

        pad(out, 8)
        node_offset = out.tell()
        copy_range(base, out, base.no, base.nc * 20)
        for rec in appended:
            out.write(rec)
        for b, rec in patched.items():
            end = out.tell()
            out.seek(node_offset + b * 20)
            out.write(rec)
            out.seek(end)

        def table(old_offset, old_count, extra):
            pad(out, 8)
            start = out.tell()
            copy_range(base, out, old_offset, old_count * 8)
            out.write(b"".join(struct.pack("<Q", o) for o in extra))
            return start

        string_table = table(base.so, base.sc, string_offsets)
        bitmap_table = table(base.bo, base.bc, bitmap_offsets)
        audio_table = table(base.ao, base.ac, audio_offsets)

        out.seek(0)
        out.write(struct.pack(HEADER, b"PKG4",
                              base.nc + len(appended), node_offset,
                              base.sc + len(strings), string_table,
                              base.bc + len(bitmaps), bitmap_table,
                              base.ac + len(audio), audio_table))


if __name__ == "__main__":
    if len(sys.argv) < 5:
        sys.exit(__doc__.strip())
    graft(sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:])
