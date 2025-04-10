//! Processed element variants

pub mod node;
pub mod relation;
pub mod way;

pub use node::*;
pub use relation::*;
pub use way::*;

pub mod common {
    use std::{
        collections::HashMap,
        hash::{Hash, Hasher},
        ops::{Add, Deref},
    };

    use crate::codec::{relation::MemberType, PrimitiveBlock};

    const VALID_ROADWAYS: [&str; 12] = [
        "motorway",
        "motorway_link",
        "trunk",
        "trunk_link",
        "primary",
        "primary_link",
        "secondary",
        "secondary_link",
        "tertiary",
        "tertiary_link",
        "residential",
        "living_street",
    ];

    #[derive(Clone, Copy, Debug, Eq, PartialOrd, Ord)]
    pub struct OsmEntryId {
        pub identifier: i64,
        variant: MemberType,
    }

    impl OsmEntryId {
        pub const fn new(id: i64, variant: MemberType) -> OsmEntryId {
            OsmEntryId {
                identifier: id,
                variant,
            }
        }

        pub const fn null() -> OsmEntryId {
            OsmEntryId {
                identifier: -1,
                variant: MemberType::Node,
            }
        }

        pub fn is_null(&self) -> bool {
            self.identifier == -1
        }

        #[inline]
        pub const fn as_node(identifier: i64) -> OsmEntryId {
            OsmEntryId {
                identifier,
                variant: MemberType::Node,
            }
        }

        #[inline]
        pub const fn as_way(identifier: i64) -> OsmEntryId {
            OsmEntryId {
                identifier,
                variant: MemberType::Way,
            }
        }
    }

    impl Add<i64> for OsmEntryId {
        type Output = OsmEntryId;

        fn add(self, other: i64) -> Self::Output {
            OsmEntryId {
                identifier: self.identifier + other,
                variant: self.variant,
            }
        }
    }

    impl From<i64> for OsmEntryId {
        // Defaults to Node variant
        fn from(value: i64) -> Self {
            OsmEntryId {
                identifier: value,
                variant: MemberType::Node,
            }
        }
    }

    impl PartialEq for OsmEntryId {
        fn eq(&self, other: &Self) -> bool {
            self.identifier == other.identifier
        }
    }

    impl Hash for OsmEntryId {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.identifier.hash(state);
        }
    }

    #[derive(Clone, Debug)]
    pub struct Role(TagString);

    #[derive(Clone, Debug)]
    pub struct Reference {
        pub id: OsmEntryId,
        role: Option<Role>,
    }

    impl Hash for Reference {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.id.hash(state);
        }
    }

    impl PartialEq for Reference {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
        }
    }

    impl Eq for Reference {}

    impl Reference {
        pub const fn new(id: OsmEntryId, role: Option<Role>) -> Self {
            Reference { id, role }
        }

        #[inline]
        pub const fn without_role(id: OsmEntryId) -> Self {
            Reference { id, role: None }
        }

        #[inline]
        pub const fn with_role(id: OsmEntryId, role: Role) -> Self {
            Reference {
                id,
                role: Some(role),
            }
        }
    }

    #[derive(Clone, Debug)]
    pub struct References(Vec<Reference>);

    /// A reference key is a tuple of the form (Role, MemberID, Type)
    pub type ReferenceKey<'a> = (&'a i32, &'a i64, &'a i32);

    pub trait Referential {
        fn indices(&self) -> impl Iterator<Item = ReferenceKey>;

        fn references(&self, block: &PrimitiveBlock) -> References {
            self.indices()
                .fold(vec![], |mut prior, (role, id, variant)| {
                    let index = id + prior.last().map_or(&0i64, |(_, v, _)| v);
                    let role = if *role == -1 {
                        None
                    } else {
                        Some(Role(TagString::recover(*role as usize, block)))
                    };

                    let member_type = MemberType::try_from(*variant).unwrap_or(MemberType::Node);

                    prior.push((role, index, member_type));
                    prior
                })
                .into_iter()
                // All nodes in a Way are `Node` types, therefore navigable.
                .map(|(role, id, variant)| Reference::new(OsmEntryId::new(id, variant), role))
                .collect::<Vec<_>>()
                .into()
        }
    }

    impl Deref for References {
        type Target = Vec<Reference>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl From<Vec<Reference>> for References {
        fn from(v: Vec<Reference>) -> Self {
            References(v)
        }
    }

    #[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Hash)]
    pub struct TagString(String);

    impl Deref for TagString {
        type Target = String;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl From<String> for TagString {
        fn from(s: String) -> Self {
            TagString(s)
        }
    }

    impl From<&str> for TagString {
        fn from(s: &str) -> Self {
            TagString(s.to_string())
        }
    }

    impl TagString {
        const HIGHWAY: &'static str = "highway";
        const ONE_WAY: &'static str = "oneway";
        const JUNCTION: &'static str = "junction";

        pub fn recover(k: usize, block: &PrimitiveBlock) -> TagString {
            TagString::from(String::from_utf8_lossy(&block.stringtable.s[k]).into_owned())
        }
    }

    #[derive(Clone, Debug)]
    pub struct Tags(HashMap<TagString, TagString>);

    pub trait Tagable {
        fn indices(&self) -> impl Iterator<Item = (&u32, &u32)>;
        fn tags(&self, block: &PrimitiveBlock) -> Tags {
            Tags::from_block(self.indices(), block)
        }
    }

    impl Tags {
        /// Takes an iterator of indicies within the string table of the
        /// associated block, and recovers the strings at the specified
        /// indexes, to generate an associative hashmap of the tag keys and values.
        ///
        /// The iterator must yield in the order of (KeyIndex, ValueIndex).
        /// This is most often implemented under the Taggable trait.
        pub fn from_block<'a>(
            iter: impl Iterator<Item = (&'a u32, &'a u32)>,
            block: &PrimitiveBlock,
        ) -> Self {
            Tags(
                iter.map(|(&k, &v)| {
                    (
                        TagString::recover(k as usize, block),
                        TagString::recover(v as usize, block),
                    )
                })
                .collect::<HashMap<TagString, TagString>>(),
            )
        }

        fn r#use(assoc: &str) -> TagString {
            TagString::from(assoc)
        }

        fn get(&self, assoc: &str) -> Option<&TagString> {
            self.0.get(&Tags::r#use(assoc))
        }

        #[inline]
        pub fn road_tag(&self) -> Option<&str> {
            self.get(TagString::HIGHWAY)
                .map(|v| v.as_str())
                .filter(|v| VALID_ROADWAYS.contains(v))
        }

        #[inline]
        pub fn one_way(&self) -> bool {
            self.get(TagString::ONE_WAY)
                .is_some_and(|v| v.as_str() == "yes")
        }

        #[inline]
        pub fn roundabout(&self) -> bool {
            self.get(TagString::JUNCTION)
                .is_some_and(|v| v.as_str() == "roundabout")
        }
    }

    impl Deref for Tags {
        type Target = HashMap<TagString, TagString>;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
}

pub use common::*;
