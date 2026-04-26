Yes—if you’re using PyO3 + the `numpy` crate, you can hand data to NumPy **without building a Python list** and even **without copying**. There are a few patterns depending on who “owns” the memory and how long it must live.

# 1) Rust hands over ownership (zero-copy, simplest)

If you can *move* your Rust container into Python and stop using it on the Rust side, use `IntoPyArray` (from the `numpy` crate). This avoids pylist construction and avoids copying.

```rust
use numpy::{IntoPyArray, PyArray1};
use pyo3::prelude::*;

// Vec<T> -> NumPy array (zero-copy move)
#[pyfunction]
fn vec_to_numpy<'py>(py: Python<'py>) -> &'py PyArray1<f32> {
    let v: Vec<f32> = (0..10_000).map(|i| i as f32).collect();
    // Moves v into a NumPy array; no Python list, no copy.
    v.into_pyarray(py)
}

// ndarray::Array<T, D> -> NumPy (zero-copy move)
use ndarray::Array2;
#[pyfunction]
fn ndarray_to_numpy<'py>(py: Python<'py>) -> &'py numpy::PyArray2<f64> {
    let arr = Array2::<f64>::zeros((1024, 256));
    arr.into_pyarray(py) // moves ownership; no copy
}
```

Notes:

* `IntoPyArray` for `Vec<T>` / `ndarray::Array` transfers ownership to Python; dropping is handled when the NumPy array is GC’d.
* This is **zero-copy** and **skips any `PyList`**.

# 2) Rust retains ownership, Python gets a view (zero-copy, lifetimed/borrowed)

If Rust must keep owning the data (e.g., a long-lived buffer you manage), you can export a **buffer** that NumPy can view via the Python buffer protocol, or build a NumPy array that borrows your memory for the lifetime of the GIL borrow.

### 2a) Borrowed NumPy view over a Rust slice (lifetime-tied)

```rust
use numpy::PyArray1;
use pyo3::prelude::*;

#[pyfunction]
fn view_over_slice<'py>(py: Python<'py>) -> &'py PyArray1<f32> {
    // some Rust-owned storage that outlives this GIL borrow
    static DATA: [f32; 4] = [1.0, 2.0, 3.0, 4.0];

    // SAFETY: We guarantee DATA stays alive and not mutated unsafely
    unsafe { PyArray1::from_slice(py, &DATA) } // creates array; may copy in older paths
}
```

`from_slice` historically copies; for strict zero-copy with retained ownership you’ll usually drop to the low-level constructor (see 3) or use the buffer protocol (2b). If you need true “no copy + Rust still owns” with correct lifetimes, prefer 2b or 3.

### 2b) Implement the Python **buffer protocol** on a `#[pyclass]`

NumPy can build an array from any object exposing a buffer:

* Implement `PyBufferProtocol` in PyO3 on a wrapper that points to your Rust memory (len, itemsize, strides).
* In Python: `np.frombuffer(rust_obj, dtype=np.float32, count=..., offset=...)` or `np.asarray(memoryview(rust_obj))`.

This gives **zero-copy** views while Rust remains the ultimate owner. Mark the buffer read-only if needed. You must ensure the underlying memory outlives any Python views.

# 3) Raw pointer + custom base (advanced, zero-copy with custom deallocator)

When you already have an allocated Rust region (e.g., `Box<[T]>`, `Arc<[T]>`, or a FFI buffer) and want NumPy to own or share it:

* Use the `numpy` crate’s low-level unsafe constructor (FFI to `PyArray_SimpleNewFromData`) to create an array from your pointer, shape, and strides.
* Attach a **base object** (often a `PyCapsule`) that holds an `Arc`/`Box` to keep the memory alive and free it when the array is collected.

Sketch:

```rust
use numpy::{PyArray, PyArrayDyn};
use pyo3::{prelude::*, types::PyCapsule};
use std::sync::Arc;

#[pyfunction]
fn from_arc_buffer<'py>(py: Python<'py>, n: usize) -> &'py PyArrayDyn<f32> {
    // allocate Rust-owned buffer and share it with Python
    let buf: Arc<[f32]> = (0..n).map(|i| i as f32).collect::<Vec<_>>().into();

    // Leak a clone into the capsule so Python holds a refcount
    let holder = buf.clone();
    let capsule = PyCapsule::new(py, holder, Some(|py, ptr| unsafe {
        // drop the Arc when capsule is destroyed
        let _ = PyCapsule::reference::<Arc<[f32]>>(py, ptr).map(|arc| std::ptr::drop_in_place(arc as *const _ as *mut _));
    })).unwrap();

    // SAFETY: Provide correct pointer, shape, and itemsize/strides
    let arr = unsafe {
        PyArray::from_raw_parts(
            py,
            &[n as _],         // shape
            &[std::mem::size_of::<f32>() as isize], // strides
            buf.as_ptr() as *mut std::ffi::c_void,
        )
    };

    // Set base so NumPy keeps capsule (and thus the Arc) alive
    unsafe { arr.set_base::<pyo3::PyAny>(capsule.into_py(py)).unwrap(); }

    arr
}
```

This fully skips lists and copies. It’s **unsafe**: you must ensure dtype/strides/pointer match and that the memory lifetime is tied to the array via the base object.

---

## Which should you pick?

* **Fastest & simplest (and you can give up ownership):** `IntoPyArray` on `Vec<T>` / `ndarray::Array` → zero-copy; no `PyList`.
* **Rust keeps owning data:** implement **buffer protocol** (2b) or build via raw pointer + base capsule (3). Both avoid `PyList` and avoid copies, but require careful lifetime/aliasing discipline.
* **Borrowed short-lived views:** acceptable for small utilities, but be careful—many convenience constructors copy; check docs or use the raw/unsafe path if you require no copy.

If you share a quick snippet of how you’re creating the arrays today (Vec, slice, `ndarray`, or custom allocator), I can show the tightest drop-in that guarantees “no PyList, no copy” for your case.
