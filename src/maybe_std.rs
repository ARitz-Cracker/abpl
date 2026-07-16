#[cfg(feature = "std")]
pub type Rc<T> = std::sync::Arc<T>;

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
pub type Rc<T> = alloc::rc::Rc<T>;
