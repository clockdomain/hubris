# Owned API Stack Optimization Guide

## Overview

The current owned API pattern for digest operations creates significant stack pressure due to large context moves. This document explores design patterns to reduce stack usage while maintaining memory safety and the benefits of owned APIs.

## Current Owned API Pattern

### Problematic Code Pattern
```rust
// Each operation moves large contexts (~757 bytes each)
let ctx = controller.init(Sha2_384)?;           // Context #1 on stack
let ctx = ctx.update(data)?;                    // Context #2, #1 moved/dropped  
let (digest, controller) = ctx.finalize()?;    // Temporary contexts during move
```

### Stack Issues
- **Multiple large contexts** exist simultaneously during moves
- **Temporary storage** during function transitions
- **Deep call stacks** amplify context copies
- **Compiler may not optimize** all intermediate moves

## Optimization Strategies

### 1. In-Place Mutation with Owned Return

**Concept**: Mutate context in-place, return ownership for chaining

```rust
pub trait DigestOpOptimized {
    fn update_in_place(mut self, data: &[u8]) -> Result<Self, Error>;
    fn finalize_in_place(mut self) -> Result<(Digest<N>, Controller), Error>;
}

impl DigestOpOptimized for OwnedDigestContext {
    fn update_in_place(mut self, data: &[u8]) -> Result<Self, Error> {
        // Mutate self directly, no intermediate context creation
        self.internal_buffer.extend_from_slice(data);
        self.process_blocks_if_needed()?;
        Ok(self) // Return moved self
    }
}

// Usage - same stack footprint throughout chain
let ctx = controller.init(Sha2_384)?;
let ctx = ctx.update_in_place(data1)?;  // Same context memory
let ctx = ctx.update_in_place(data2)?;  // Same context memory  
let (digest, ctrl) = ctx.finalize_in_place()?;
```

**Benefits**:
- **Single context instance** throughout operation chain
- **No intermediate context copies**
- **Maintains owned semantics** for safety

### 2. Context Reference with Owned Controller

**Concept**: Keep contexts as references, only move controller

```rust
pub struct DigestSession<'a> {
    context: &'a mut AspeedHashContext,
    controller: Option<HaceController>,
}

impl<'a> DigestSession<'a> {
    pub fn update(&mut self, data: &[u8]) -> Result<(), Error> {
        // Work directly on referenced context
        self.context.buffer[..data.len()].copy_from_slice(data);
        self.process_if_needed()
    }
    
    pub fn finalize(mut self) -> Result<(Digest<N>, HaceController), Error> {
        // Only move controller, not context
        let controller = self.controller.take().unwrap();
        let digest = self.compute_final_digest()?;
        Ok((digest, controller))
    }
}
```

**Benefits**:
- **No context moves** during operations
- **Large contexts remain stationary** in memory
- **Controller ownership** still managed safely

### 3. Statically Allocated Context Pool

**Concept**: Pre-allocate contexts in static memory, use indices

```rust
// In non-cacheable static memory
#[link_section = ".noncacheable_bss"]
static mut CONTEXT_POOL: [AspeedHashContext; MAX_SESSIONS] = 
    [AspeedHashContext::new(); MAX_SESSIONS];

static POOL_ALLOCATOR: Mutex<[bool; MAX_SESSIONS]> = 
    Mutex::new([false; MAX_SESSIONS]);

pub struct OwnedContextHandle {
    index: usize,
    _phantom: PhantomData<AspeedHashContext>,
}

impl OwnedContextHandle {
    pub fn allocate() -> Result<Self, Error> {
        let mut allocator = POOL_ALLOCATOR.lock();
        for (i, &used) in allocator.iter().enumerate() {
            if !used {
                allocator[i] = true;
                return Ok(OwnedContextHandle { 
                    index: i, 
                    _phantom: PhantomData 
                });
            }
        }
        Err(Error::NoAvailableContext)
    }
    
    fn context_mut(&mut self) -> &mut AspeedHashContext {
        unsafe { &mut CONTEXT_POOL[self.index] }
    }
}

impl Drop for OwnedContextHandle {
    fn drop(&mut self) {
        let mut allocator = POOL_ALLOCATOR.lock();
        allocator[self.index] = false;
    }
}
```

**Benefits**:
- **Zero stack allocation** for contexts
- **Non-cacheable memory** placement for DMA coherency
- **Automatic cleanup** via Drop trait

### 4. Boxed Contexts in Non-Cacheable Heap

**Concept**: Custom allocator for non-cacheable memory

```rust
use linked_list_allocator::LockedHeap;

// Custom allocator for non-cacheable region
#[link_section = ".noncacheable_bss"]
static mut NONCACHEABLE_HEAP: [u8; 16384] = [0; 16384];

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

// Box-like wrapper for non-cacheable allocation
pub struct NonCacheableBox<T> {
    ptr: NonNull<T>,
    _phantom: PhantomData<T>,
}

impl<T> NonCacheableBox<T> {
    pub fn new(value: T) -> Result<Self, Error> {
        let layout = Layout::new::<T>();
        let ptr = unsafe { 
            ALLOCATOR.alloc(layout).map_err(|_| Error::OutOfMemory)?
        };
        let typed_ptr = ptr.cast::<T>();
        unsafe { typed_ptr.as_ptr().write(value) };
        
        Ok(NonCacheableBox {
            ptr: typed_ptr,
            _phantom: PhantomData,
        })
    }
}

// Usage
let ctx_box = NonCacheableBox::new(AspeedHashContext::default())?;
```

**Benefits**:
- **Heap allocation** removes stack pressure completely  
- **Non-cacheable placement** ensures DMA coherency
- **Custom allocator** maintains no_std compatibility

## Implementation Recommendations

### Phase 1: In-Place Updates (Low Risk)
1. **Modify existing owned API** to use in-place mutations
2. **Maintain same external interface** for compatibility
3. **Test stack usage reduction** without architectural changes

### Phase 2: Static Context Pool (Medium Risk)
1. **Implement static context pool** in non-cacheable memory
2. **Create owned handle types** for automatic management
3. **Migrate session management** to use pool indices

### Phase 3: Custom Non-Cacheable Allocator (High Risk)
1. **Implement custom allocator** for non-cacheable region
2. **Create Box-like abstractions** for owned contexts
3. **Full heap-based context management**

## Stack Usage Comparison

| Approach | Stack Usage | DMA Coherency | Implementation Complexity |
|----------|-------------|---------------|---------------------------|
| Current Owned API | ~9KB | ❌ Issues | ✅ Simple |
| In-Place Updates | ~1KB | ❌ Issues | ✅ Low |
| Static Pool | ~256B | ✅ Correct | ⚠️ Medium |
| Non-Cacheable Heap | ~128B | ✅ Correct | 🔴 High |

## Code Examples

### Before: Current Implementation
```rust
// High stack usage - multiple context copies
impl<D> ServerImpl<D> {
    fn finalize_sha256_internal(&mut self, session_id: u32) -> Result<[u32; 8], Error> {
        let mut session = self.sessions.remove(&session_id)?; // Stack copy #1
        match &mut session.context {
            SessionContext::Sha256(ctx_opt) => {
                let ctx = ctx_opt.take()?;                    // Stack copy #2
                let (digest, controller) = ctx.finalize()?;  // Stack copy #3 during move
                // ... rest of function
            }
        }
    }
}
```

### After: In-Place Updates
```rust
// Low stack usage - single context throughout
impl<D> ServerImpl<D> {
    fn finalize_sha256_internal(&mut self, session_id: u32) -> Result<[u32; 8], Error> {
        let session = self.sessions.get_mut(&session_id)?;   // Borrow only
        match &mut session.context {
            SessionContext::Sha256(ctx_ref) => {
                let (digest, controller) = ctx_ref.finalize_in_place()?; // No copies
                self.controllers.hardware = Some(controller);
                Ok(digest.into_array())
            }
        }
    }
}
```

### After: Static Pool
```rust
// Minimal stack usage - handle-based access
impl<D> ServerImpl<D> {
    fn finalize_sha256_internal(&mut self, session_id: u32) -> Result<[u32; 8], Error> {
        let session = self.sessions.remove(&session_id)?;
        match session.context {
            SessionContext::Sha256(handle) => {              // Handle is small
                let (digest, controller) = handle.finalize()?; // No context moves
                self.controllers.hardware = Some(controller);
                Ok(digest.into_array())
            }
        }
    }
}
```

## Testing Strategy

### Stack Usage Measurement
1. **Before/after profiling** with different approaches
2. **Worst-case scenario testing** with maximum concurrent sessions
3. **Call stack depth analysis** under various load conditions

### Correctness Validation  
1. **DMA coherency tests** with hardware operations
2. **Multi-session concurrent testing** for race conditions
3. **Memory leak detection** for pool/allocator implementations

### Performance Benchmarking
1. **Throughput comparison** between approaches
2. **Latency measurement** for different context management strategies
3. **Memory access pattern analysis** (cacheable vs non-cacheable)

## Migration Path

### Step 1: Proof of Concept
- **Implement in-place updates** for single algorithm
- **Measure stack reduction** and verify functionality
- **Assess implementation effort** for full migration

### Step 2: Incremental Rollout
- **Migrate one API method** at a time
- **Maintain backward compatibility** during transition
- **Performance test** each change

### Step 3: Full Implementation
- **Complete owned API refactoring**
- **Implement static pool** or custom allocator
- **Remove legacy implementation** once validated

## Conclusion

**Recommended approach**: Start with **in-place updates** for immediate stack reduction with minimal risk, then evaluate **static context pool** for long-term solution addressing both stack usage and DMA coherency requirements.

The static pool approach offers the best balance of:
- **Dramatic stack reduction** (~99% less stack usage)
- **DMA coherency compliance** (non-cacheable placement)  
- **Reasonable implementation complexity**
- **Maintained safety guarantees** through owned handles