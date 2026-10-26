//! The ReBirth RB-338 2.01 audio engine, reimplemented.
//!
//! # Numerics rules
//!
//! The output must match the original sample for sample, so every module follows these rules
//! (`docs/initial_investigation.md`, section 4, "Numerics rules"):
//!
//! - The original's x87 code runs at 53-bit precision, so it behaves like IEEE f64. Compute in
//!   `f64`.
//! - Round to `f32` exactly where the original stores a float to memory. Many loops keep state in
//!   x87 registers for the whole block and store an `f32` only at the block end.
//! - Never use fused multiply-add (`f64::mul_add` and friends), and keep the operand order of the
//!   original.
//! - Float to integer conversions truncate toward zero (`as i32` / `as i64` on in-range values).
//! - The original's transcendentals (`fcos`, `fsin`, `f2xm1` + `fscale`, and the CRT `pow`) run at
//!   x87 extended precision. They are used only to build tables at init, and every table is
//!   stored as `f32`. The plan (task-019) is to generate the tables with f64 maths, diff them
//!   against dumps of the real tables, and embed the dumped tables as binary blobs if any `f32`
//!   entry differs.

pub mod load;
pub mod song;
