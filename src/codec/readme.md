### `codec`

Split into multiple segments:
1. Blob
2. Block
3. Element

The `OSM` protobuf structure sorts elements as such,
where this component provides each segment's iterator,
to allow for more control over each level, in terms 
of permitting parallel processing, injections, etc.

```text
blob item
  └─┬─ block item - Two Block Variants
    ├─┬─  header block    
    │ ├── ... metadata
    │ └── bounding box
    │ 
    └─┬─ primitive block
      ├── ... metadata
      └─┬─ primitive group[] - Elements
        ├── Node[]
        ├── DenseNodes
        ├── Way[]
        └── Relation[]
```

The above structure outlines the `blob`, `block` 
and `element` subsections of the `codec` module. 

#### Blob
The `blob item`, outlines the `blob` section. This refers 
to the `BlobItem` structure which sections to the size and
length of each file block it contains, held in the `BlobHeader`
of which it controls.

A `BlobHeader` can have two types:
- "OSMData"
- "OSMHeader"

An `OSMHeader` blob refers to a `header block` on the above 
diagram, whilst an `OSMData` blob is a `primitive block`.

These blobs can be used to quickly index the file, removing
the need to parse the entire header or data structure. This means
we can utilize parallel optimisation to parse our data blocks,
whilst knowing their file offset, similar to a 
[Skip List](https://en.wikipedia.org/wiki/Skip_list?useskin=vector).

#### Block
In order to parse a block, we need to know the offset and size
of the block, which is conveniently stored in a `BlobHeader`.

This `BlobHeader` has a `data` field, which contains the data
of our block, the type of block we are decoding is stored as
a literal string, `OSMData` or `OSMHeader`. 

All blocks can be decoded using the `block` module, enumerable
using the `BlockIterator`, which enumerates over each block.

This iterator will return a `BlockItem`, which can be either
a `PrimitiveBlock` or a `HeaderBlock`.

In order to decode each utility item, such as a `Node` or `Way`,
an iteration over the `BlockItem`'s elements is required.

For understanding context, bounds, etc. Use of the `HeaderBlock`
is required.

