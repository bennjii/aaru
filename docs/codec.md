Defines parser logic and iterators over `OSM` elements.

### Iterators

At your disposal there are:
- The [`BlobIterator`] - Iterate over `.osm.pbf` blob segments
- The [`BlockIterator`] - Iterate over `Header`/`Primitive` blocks
- The [`ElementIterator`] - Iterate over un-decoded `Node`, `Way`, `Relation` and `DenseNodes` set. 
- The [`ProcessedElementIterator`] - Iterate over decoded `Node` and `Way`s.

Each of which can be done in series, or in parallel wherever the `Parallel` trait is implemented.

### Encoding

To understand when to use what iterator, we can understand it as the following.

The file is split into multiple depth levels:
1. Blob - Highest depth (contains multiple elements)
2. Block - Medium depth
3. Element - Lowest depth

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

The above structure outlines the [`blob`], [`block`]
and [`element`] subsections of the `codec` module.

#### Blob
The `blob item`, outlines the [`blob`] section. This refers
to the [`BlobItem`] structure which sections to the size and
length of each file block it contains, held in the [`BlobHeader`]
of which it controls.

A [`BlobHeader`] can have two types:
- `"OSMData"` - Data Subcomponent
- `"OSMHeader"` - Metadata Subcomponent

An `OSMHeader` blob refers to a `header block` on the above
diagram, whilst an `OSMData` blob is a `primitive block`.
These blobs are considered "lightweight", as they contain
small data headers, and simply indicate where the next header
is, so can be used to quickly *index* the file, removing
the need to parse the entire data structure it contains.

This means we can utilize parallel optimisation to parse our data blocks,
whilst knowing their file offset, similar to a [Skip List](https://en.wikipedia.org/wiki/Skip_list?useskin=vector).

#### Block
In order to parse a block, we need to know the offset and size
of the block, which is conveniently stored in a [`BlobHeader`].

This [`BlobHeader`] has a `data` field, which contains the data
of our block, the type of block we are decoding is stored as
a literal string, `"OSMData"` or `"OSMHeader"`.

All blocks can be decoded using the `block` module, enumerable
using the [`BlockIterator`], which enumerates over each block.
This iterator will return a [`BlockItem`], which can be either
a [`PrimitiveBlock`] or a [`HeaderBlock`].

```rust,no_run
let path = PathBuf::from(DISTRICT_OF_COLUMBIA);
let iterator = BlockIterator::new(path)
    .expect("Failed to create iterator");

for block in iter {
    // Do something with the block...
}
```

In order to decode each utility item, such as a [`Node`] or [`Way`],
an iteration over the [`BlockItem`]'s elements is required, which
is done through the [`ElementIterator`].

In order to determine where nodes are positioned, contained,
offset, etc. We utilise the [`HeaderBlock`] component, which
contains such data. More information can be found from the
OSM wiki itself, [here](https://wiki.openstreetmap.org/wiki/PBF_Format#Definition_of_the_OSMHeader_fileblock).

#### Element

Lastly, we have elements themselves. These are served in two
variants. The first has no extra parsing performed, this is
more suited to applications which do not require 