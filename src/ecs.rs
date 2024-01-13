// Credit for this implementation outline to Kyren https://kyren.github.io/2018/09/14/rustconf-talk.html

pub type IndexType = u16;
pub type GenerationType = u16;

// You can use other types that usize / u64 if these are too large
#[derive(Eq, PartialEq)]
pub struct GenerationalIndex {
    index: IndexType,
    generation: GenerationType,
}

pub struct AllocatorEntry {
    pub is_live: bool,
    pub generation: GenerationType,
}

pub struct GenerationalIndexAllocator {
    pub entries: Vec<AllocatorEntry>,
    pub free: Vec<IndexType>,
    pub generation_counter: GenerationType,
}

pub struct AllocationFailed(());

#[derive(Debug)]
pub enum DeallocationError {
    IndexOOB,
    GenerationMismatch,
    AlreadyDeallocated
}

// pub struct LiveLookupOOB(());

impl GenerationalIndexAllocator {

    pub fn allocate(&mut self) -> Result<GenerationalIndex, AllocationFailed> {
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
            None => Err(AllocationFailed(())),
        }
    }



    // Return index back to pool of available ones. This does NOT deallocate the resource itself
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
    
    // pub fn is_live(&self, index: GenerationalIndex) -> Result<bool, LiveLookupOOB> {
    //     if index.index >= self.entries.len() {
    //         Err(LiveLookupOOB(()))
    //     } else {
    //         Ok(self.entries[index.index].is_live)
    //     }
    // }
}

pub struct ArrayEntry<T> {
    pub value: T,
    pub generation: GenerationType,
}

// An associative array from GenerationalIndex to some Value T.
pub struct GenerationalIndexArray<T>(pub Vec<Option<ArrayEntry<T>>>);

pub struct IndexOOB(());

impl<T> GenerationalIndexArray<T> {
    // Set the value for some generational index.  May overwrite past generation
    // values.
    pub fn set(&mut self, index: &GenerationalIndex, value: T) -> Result<(), IndexOOB> {
        if index.index >= self.0.len() as IndexType {
            Err(IndexOOB(()))
        } else {
            self.0[index.index as usize] = Some(ArrayEntry{value, generation: index.generation});
            Ok(())
        }
    }

    // Gets the value for some generational index, the generation must match.
    pub fn get(&self, index: &GenerationalIndex) -> Option<&T> {
        if index.index >= self.0.len() as IndexType {
            None
        } else {
            match &self.0[index.index as usize] {
                Some(ae) => {
                    if index.generation != ae.generation {
                        None
                    } else {
                        Some(&ae.value)
                    }
                },
                None => None,
            }
        }
    }

    // Get the value of some generational index. The generation must match.
    pub fn get_mut(&mut self, index: &GenerationalIndex) -> Option<&mut T> {
        if index.index >= self.0.len() as IndexType {
            None
        } else {
            match &mut self.0[index.index as usize] {
                Some(ae) => {
                    if index.generation != ae.generation {
                        None
                    } else {
                        Some(&mut ae.value)
                    }
                },
                None => None,
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

