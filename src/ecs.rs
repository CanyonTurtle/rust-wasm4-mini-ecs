// Credit for this implementation outline to Kyren https://kyren.github.io/2018/09/14/rustconf-talk.html

pub type IndexType = u16;
pub type GenerationType = u32;

/// Represent an index that always points to a small number in a vector, but also has a generation that allows it to expire. 
/// You can change this struct's internal size types if these are too large.
#[derive(Eq, PartialEq, Clone, Copy)]
pub struct GenerationalIndex {
    index: IndexType,
    generation: GenerationType,
}

/// Represent available spots in the generational allocator. This stays public even though it's really for internal use, so that the allocation for these happens upfront explicitly (see demo usage).
pub struct AllocatorEntry {
    is_live: bool,
    generation: GenerationType,
}

impl AllocatorEntry {
    pub fn new()-> AllocatorEntry {
        AllocatorEntry {
            is_live: false,
            generation: 0,
        }
    }
}

/// Represent which indecies are currently in use by which generation, and handle allocation and deallocation of these indecies.
/// This does NOT allocate the actual data stored in the entity component system, JUST the indecies.
/// This is on purpose; it allows manual management of the component memory by the user.
pub struct GenerationalIndexAllocator {
    entries: Vec<AllocatorEntry>,
    free: Vec<IndexType>,
    generation_counter: GenerationType,
}

impl GenerationalIndexAllocator {
    pub fn new(entries: Vec<AllocatorEntry>, free: Vec<IndexType>) -> GenerationalIndexAllocator {
        GenerationalIndexAllocator {
            entries,
            free,
            generation_counter: 0,
        }
    }
}
pub struct AllocatorOutOfMemory(());

#[derive(Debug)]
pub enum DeallocationError {
    IndexOOB,
    GenerationMismatch,
    AlreadyDeallocated
}

#[derive(Debug)]
pub enum GenerationalIndexError {
    IndexOOB,
    GenerationMismatch,
    NotLive
}

impl GenerationalIndexAllocator {

    /// Reserve some index and return it as a handle to be used with GenerationalIndexArrays (and to be deallocated later).
    pub fn allocate(&mut self) -> Result<GenerationalIndex, AllocatorOutOfMemory> {
        // try to find a free spot.

        match self.free.pop() {
            Some(index) => {
                self.generation_counter += 1;
                self.entries[index as usize].generation = self.generation_counter;
                self.entries[index as usize].is_live = true;
                Ok(GenerationalIndex{
                    index: index as IndexType,
                    generation: self.generation_counter
                }) 
            },
            None => Err(AllocatorOutOfMemory(())),
        }
    }



    /// Return index back to pool of available ones. This does NOT deallocate the resource itself.
    pub fn deallocate(&mut self, index: &GenerationalIndex) -> Result<(), DeallocationError> {
        let i = index.index;
        if i >= self.entries.len() as IndexType {
            Err(DeallocationError::IndexOOB)
        } else if self.entries[i as usize].generation != index.generation {
            Err(DeallocationError::GenerationMismatch)
        } else if !self.entries[i as usize].is_live {
            Err(DeallocationError::AlreadyDeallocated)
        } else {
            self.entries[i as usize].is_live = false;
            self.free.push(i);
            Ok(())
        }
    }
    
    /// Check whether this index is live (i.e. if it was deallocated, the index still exists, but it's not "live").
    pub fn is_live(&self, index: &GenerationalIndex) -> Result<bool, GenerationalIndexError> {
        if index.index >= self.entries.len() as IndexType {
            Err(GenerationalIndexError::IndexOOB)
        } else {
            Ok(self.entries[index.index as usize].is_live)
        }
    }
}

// An associative array from GenerationalIndex to some Value T. Since get, set, and get_mut require the allocator to be passed in,
// the datatype doesn't require anything to be stored in these arrays themselves.
pub struct GenerationalIndexArray<T>(pub Vec<T>);

impl<T> GenerationalIndexArray<T> {
    // Set the value for some generational index, the generation must match AND this index must be live in the passed-in allocator.
    pub fn set(&mut self, index: &GenerationalIndex, allocator: &GenerationalIndexAllocator, value: T) -> Result<(), GenerationalIndexError> {
        if index.index >= self.0.len() as IndexType {
            Err(GenerationalIndexError::IndexOOB)
        } else {
            match allocator.is_live(&index) {
                Ok(alive) => match alive {
                    true => {
                        if index.generation != allocator.entries[index.index as usize].generation {
                            Err(GenerationalIndexError::GenerationMismatch)
                        } else {
                            self.0[index.index as usize] = value;
                            Ok(())
                        }
                    },
                    false => Err(GenerationalIndexError::NotLive)
                }
                Err(e) => Err(e),
            }
        }
    }

    /// Gets the value for some generational index, the generation must match AND this index must be live in the passed-in allocator.
    pub fn get(&self, index: &GenerationalIndex, allocator: &GenerationalIndexAllocator) -> Result<&T, GenerationalIndexError> {
        if index.index >= self.0.len() as IndexType {
            Err(GenerationalIndexError::IndexOOB)
        } else {
            match allocator.is_live(&index) {
                Ok(alive) => match alive {
                    true => {
                        let val = &self.0[index.index as usize];
                        if allocator.entries[index.index as usize].generation != index.generation {
                            Err(GenerationalIndexError::GenerationMismatch)
                        } else {
                            Ok(val)
                        }
                    },
                    false => Err(GenerationalIndexError::NotLive)
                }
                Err(e) => Err(e),
            }
        }   
    }

    /// Mutably gets the value for some generational index, the generation must match AND this index must be live in the passed-in allocator.
    pub fn get_mut(&mut self, index: &GenerationalIndex, allocator: &GenerationalIndexAllocator) -> Result<&mut T, GenerationalIndexError> {
        if index.index >= self.0.len() as IndexType {
            Err(GenerationalIndexError::IndexOOB)
        } else {
            match allocator.is_live(&index) {
                Ok(alive) => match alive {
                    true => {
                        let val = &mut self.0[index.index as usize];
                        if index.generation != allocator.entries[index.index as usize].generation {
                            Err(GenerationalIndexError::GenerationMismatch)
                        } else {
                            Ok(val)
                        }
                    },
                    false => Err(GenerationalIndexError::NotLive)
                }
                Err(e) => Err(e),
            }
        }   
    }
}

// We're dropping the index or id suffix, because there is no other "Entity"
// type to get confused with.  Don't forget though, this doesn't "contain"
// anything, it's just a sort of index or id or handle or whatever you want to
// call it.
pub type Entity = GenerationalIndex;

// Map of Entity to some type T
pub type EntityMap<T> = GenerationalIndexArray<T>;

