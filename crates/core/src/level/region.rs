// use std::iter::FusedIterator;
// use std::ops::Range;
// use std::sync::Arc;
// use rayon::iter::plumbing::{bridge, Consumer, Producer, ProducerCallback, UnindexedConsumer};
// use rayon::prelude::{IndexedParallelIterator, ParallelIterator};
// use tokio::sync::mpsc;
// use level::provider::Provider;
// use level::SubChunk;
// use util::Vector;
// use crate::level::{Request, Service};

// struct RegionProducer {
//     xrange: Range<i32>,
//     yrange: Range<i32>,
//     zrange: Range<i32>,
//     provider: Arc<Provider>
// }

// impl Producer for RegionProducer {
//     type Item = SubChunk;
//     type IntoIter = RegionQueryIter;

//     fn into_iter(self) -> Self::IntoIter {
//         RegionQueryIter { 
//             provider: self.provider,
//             query: RegionQuery::from_ranges(self.xrange, self.yrange, self.zrange)
//         }
//     }

//     fn split_at(self, index: usize) -> (Self, Self) {
        
//     }
// }

// struct RegionQueryIter {
//     query: RegionQuery,
//     index_front: usize,
//     index_back: usize,
//     provider: Arc<Provider>
// }

// impl ParallelIterator for RegionQueryIter {
//     type Item = SubChunk;

//     #[inline]
//     fn drive_unindexed<C>(self, consumer: C) -> C::Result
//         where C: UnindexedConsumer<Self::Item>
//     {
//         bridge(self, consumer)
//     }
// }

// impl IndexedParallelIterator for RegionQueryIter {
//     #[inline]
//     fn len(&self) -> usize {
//         self.query.len()
//     }

//     #[inline]
//     fn drive<C>(self, consumer: C) -> C::Result
//     where
//         C: Consumer<Self::Item>
//     {
//         bridge(self, consumer)
//     }

//     #[inline]
//     fn with_producer<CB>(self, callback: CB) -> CB::Output
//     where
//         CB: ProducerCallback<Self::Item>
//     {
//         todo!()
//     }
// }

// impl ExactSizeIterator for RegionQueryIter {
//     fn len(&self) -> usize {
//         self.index_back - self.index_front
//     }
// }

// /// Requests all chunks in a rectangle between two bounds.
// pub struct RegionQuery {
//     xrange: Range<i32>,
//     yrange: Range<i32>,
//     zrange: Range<i32>,
//     provider: Arc<Provider>
// }

// impl RegionQuery {
//     #[inline]
//     pub fn from_ranges(
//         xrange: Range<i32>, yrange: Range<i32>, zrange: Range<i32>, provider: Arc<Provider>
//     ) -> RegionQuery {
//         Self { xrange, yrange, zrange, provider }
//     }

//     /// The amount of chunks this query will request.
//     #[inline]
//     pub fn len(&self) -> usize {
//         self.xrange.len() * self.yrange.len() * self.zrange.len()
//     }

//     fn from_index(&self, mut index: usize) -> Vector<i32, 3> {
//         let x = index as i32 % (self.xrange.len() as i32);
//         index /= self.xrange.len() as usize;

//         let y = index as i32 % (self.yrange.len() as i32);
//         index /= self.yrange.len() as usize;

//         let z = index as i32;

//         Vector::from([x, y, z])
//     }
// }

// impl Request for RegionQuery {
//     type Output = ();

//     fn execute(mut self, service: Arc<Service>) -> mpsc::Receiver<Self::Output> {
//         let (sender, receiver) = mpsc::channel(self.len());


//         receiver
//     }
// }

use std::{iter::FusedIterator, ops::Range, sync::Arc};

use level::{provider::Provider, SubChunk};
use rayon::iter::{plumbing::{bridge, Consumer, Producer, ProducerCallback, UnindexedConsumer}, IndexedParallelIterator, ParallelIterator};
use tokio::sync::mpsc;
use util::Vector;

use crate::instance::Instance;

use super::{Request, Service};

/// Produces split region iterators.
pub struct RegionProducer(RegionIter);

impl Producer for RegionProducer {
    type Item = Vector<i32, 3>;
    type IntoIter = RegionIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        RegionIter::from(self)
    }

    #[inline]
    fn split_at(self, index: usize) -> (Self, Self) {
        let left = Self(RegionIter {
            region: self.0.region.clone(),
            front_index: 0,
            back_index: index,
            provider: Arc::clone(&self.0.provider)
        });

        let right = Self(RegionIter {
            region: self.0.region,
            front_index: index,
            back_index: self.0.back_index,
            provider: self.0.provider
        });

        (left, right)
    }
}

impl From<RegionIter> for RegionProducer {
    #[inline]
    fn from(value: RegionIter) -> Self {
        Self(value)
    }
}

/// An iterator that iterates over every single subchunk coordinate within a region.
pub struct RegionIter {    
    region: RegionQuery,
    front_index: usize,
    back_index: usize,
    provider: Arc<Provider>
}

impl ParallelIterator for RegionIter {
    type Item = Vector<i32, 3>;

    #[inline]
    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>
    {
        bridge(self, consumer)
    }

    #[inline]
    fn opt_len(&self) -> Option<usize> {
        Some(self.region.len())
    }
}

impl IndexedParallelIterator for RegionIter {
    #[inline]
    fn with_producer<CB>(self, callback: CB) -> CB::Output 
    where
        CB: ProducerCallback<Self::Item>
    {
        let producer = RegionProducer::from(self);
        callback.callback(producer)
    }

    #[inline]
    fn drive<C>(self, consumer: C) -> C::Result
    where
        C: Consumer<Self::Item>
    {
        bridge(self, consumer)
    }

    #[inline]
    fn len(&self) -> usize {
        self.region.len()
    }
}

impl Iterator for RegionIter {
    type Item = Vector<i32, 3>;

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.front_index += n;
        self.next()
    }

    fn next(&mut self) -> Option<Self::Item> {
        (self.len() > 0).then(|| {
            self.front_index += 1;
            self.region.get_unchecked(self.front_index - 1)
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(ExactSizeIterator::len(self)))
    }
}

impl ExactSizeIterator for RegionIter {
    #[inline]
    fn len(&self) -> usize {
        // Use checked subtraction to make sure the length does not overflow
        // when back_index < front_index.
        self.front_index.checked_sub(self.back_index).unwrap_or(0)
    }
}

impl FusedIterator for RegionIter {}

impl DoubleEndedIterator for RegionIter {
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        // Unlike `nth`, this can overflow if we are already at 0.
        self.back_index.checked_sub(n)?;
        self.next_back()
    }
    
    fn next_back(&mut self) -> Option<Self::Item> {
        (self.len() > 0).then(|| {
            self.back_index -= 1;
            self.region.get_unchecked(self.back_index + 1)
        })
    }
}

impl From<RegionProducer> for RegionIter {
    #[inline]
    fn from(producer: RegionProducer) -> Self {
        producer.0
    }
}

/// A query that requests a certain region of subchunks from the level provider.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionQuery {
    xrange: Range<i32>,
    yrange: Range<i32>,
    zrange: Range<i32>
}

impl RegionQuery {
    /// Creates an iterator over this region using the given level provider.
    pub fn iter(&self, provider: Arc<Provider>) -> RegionIter {
        RegionIter {
            provider,
            front_index: 0,
            back_index: self.len(),
            region: self.clone()
        }
    }

    /// Creates a region query using two corner coordinates.
    /// 
    /// The region will represent the box between these two corners.
    /// The given bounds should be in subchunk coordinates.
    pub fn from_bounds<B1, B2>(bound1: B1, bound2: B2) -> Self 
    where
        B1: Into<Vector<i32, 3>>,
        B2: Into<Vector<i32, 3>>
    {
        let bound1 = bound1.into();
        let bound2 = bound2.into();

        Self::from_bounds_inner(bound1, bound2)
    }

    /// Converts an index to a coordinate within this region, without checking
    /// for bounds.
    /// 
    /// This function is not marked as unsafe because incorrect input does not cause memory unsafety.
    /// Using an index out of bounds will simply return a coordinate outside of the region.
    /// However, the coordinate will likely be incorrect because different regions use incompatible indices.
    pub fn get_unchecked(&self, mut index: usize) -> Vector<i32, 3> {
        let x = index as i32 % (self.xrange.len() as i32);
        index /= self.xrange.len() as usize;

        let y = index as i32 % (self.yrange.len() as i32);
        index /= self.yrange.len() as usize;

        let z = index as i32;

        Vector::from([x, y, z])
    }

    /// Converts a coordinate to an index within this region, without checking 
    /// for bounds.
    /// 
    /// This function is not marked as unsafe because incorrect input does not cause memory unsafety.
    /// Using a coordinate out of bounds will simply return a index outside of the region.
    /// However, the index will likely be incorrect because different regions use incompatible indices.
    pub fn index_unchecked(&self, coord: Vector<i32, 3>) -> usize {
        todo!()
    }

    /// Converts a coordinate within this region to an index, ensuring
    /// that the coordinate is not out of bounds.
    pub fn index(&self, coord: Vector<i32, 3>) -> Option<usize> {
        todo!()
    }

    /// Converts an index to a coordinate within this region, ensuring
    /// that the index is not out of bounds.
    pub fn get(&self, index: usize) -> Option<Vector<i32, 3>> {
        (index <= self.len()).then(|| self.get_unchecked(index))
    }

    /// The amount of subchunks contained in this region.
    pub fn len(&self) -> usize {
        self.xrange.len() * self.yrange.len() * self.zrange.len()
    }

    fn from_bounds_inner(bound1: Vector<i32, 3>, bound2: Vector<i32, 3>) -> Self {
        let xmin = std::cmp::min(bound1.x, bound2.x);
        let xmax = std::cmp::min(bound1.x, bound2.x);
        let xrange = xmin..xmax;

        let ymin = std::cmp::min(bound1.y, bound2.y);
        let ymax = std::cmp::min(bound1.y, bound2.y);
        let yrange = ymin..ymax;

        let zmin = std::cmp::min(bound1.z, bound2.z);
        let zmax = std::cmp::min(bound1.z, bound2.z);
        let zrange = zmin..zmax;

        Self {
            xrange, yrange, zrange
        }
    }
}

impl Request for RegionQuery {
    type Output = mpsc::Receiver<SubChunk>;

    fn execute(self, service: &Arc<Service>) -> Self::Output {
        let (sender, receiver) = mpsc::channel(self.len());
        let iter = RegionIter {
            provider: Arc::clone(&service.provider),
            front_index: 0,
            back_index: self.len() - 1,
            region: self
        };

        todo!();

        receiver
    }
}