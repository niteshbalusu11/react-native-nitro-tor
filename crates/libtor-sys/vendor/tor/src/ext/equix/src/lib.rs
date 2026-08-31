//! Rust wrapper for Equi-X and HashX
//!
//! This is a Rust wrapper for the original C implementation of Equi-X and
//! HashX, as used by the C implementation of Tor. For cross-compatibility
//! testing conducted by Arti.
//!
//! The wrapper statically links with a modified version of the original
//! implementation by tevador, covered by the LGPL version 3. This modified
//! codebase is maintained as an ext module within the tor source distribution.
//!
//! Equi-X and HashX are `Copyright (c) 2020 tevador <tevador@gmail.com>`.
//! See `LICENSE` for licensing information.
//!

use core::ffi::c_void;
use core::mem;
use core::ptr::null_mut;

pub mod ffi {
    //! Low-level access to the C API

    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

/// Type parameter for [`HashX::new()`]
pub type HashXType = ffi::hashx_type;

/// Result codes for HashX
pub type HashXResult = ffi::hashx_result;

/// Configured size of the HashX output. Always 8 in this implementation.
pub const HASHX_SIZE: usize = ffi::HASHX_SIZE as usize;

/// Output value obtained by executing a HashX hash function
pub type HashXOutput = [u8; HASHX_SIZE];

/// Type for callback functions that inspect or replace the pseudorandom stream
pub type RngCallback = Box<dyn FnMut(u64) -> u64>;

/// Safe wrapper around a HashX context
pub struct HashX {
    ctx: *mut ffi::hashx_ctx,
    rng_callback: Option<RngCallback>,
}

impl HashX {
    /// Allocate a new HashX context
    pub fn new(ht: HashXType) -> Self {
        let ctx = unsafe { ffi::hashx_alloc(ht) };
        if ctx.is_null() {
            panic!("out of memory in hashx_alloc");
        }
        Self {
            ctx,
            rng_callback: None,
        }
    }

    /// Create a new hash function within this context, using the given seed
    ///
    /// May fail if the seed is unusable or if a runtime compiler
    /// error occurs while the interpreter is disabled.
    #[inline(always)]
    pub fn make(&mut self, seed: &[u8]) -> HashXResult {
        unsafe { ffi::hashx_make(self.ctx, seed.as_ptr() as *const c_void, seed.len()) }
    }

    /// Check which implementation was selected by `make`
    #[inline(always)]
    pub fn query_type(&mut self) -> Result<HashXType, HashXResult> {
        let mut buffer = HashXType::HASHX_TYPE_INTERPRETED; // Arbitrary default
        let result =
            unsafe { ffi::hashx_query_type(self.ctx, &mut buffer as *mut ffi::hashx_type) };
        match result {
            HashXResult::HASHX_OK => Ok(buffer),
            e => Err(e),
        }
    }

    /// Execute the hash function for a given input
    #[inline(always)]
    pub fn exec(&mut self, input: u64) -> Result<HashXOutput, HashXResult> {
        let mut buffer: HashXOutput = Default::default();
        let result =
            unsafe { ffi::hashx_exec(self.ctx, input, &mut buffer as *mut u8 as *mut c_void) };
        match result {
            HashXResult::HASHX_OK => Ok(buffer),
            e => Err(e),
        }
    }

    /// Set a callback function that may inspect and/or modify the internal
    /// pseudorandom number stream used by this context.
    ///
    /// The function will be owned by this context, and it replaces any
    /// previous function that may have been set. Returns the previous callback
    /// if any.
    pub fn rng_callback(&mut self, callback: Option<RngCallback>) -> Option<RngCallback> {
        // Keep ownership of our Rust value in the context wrapper, to match
        // the lifetime of the mutable pointer that the C API saves.
        let result = mem::replace(&mut self.rng_callback, callback);
        match &mut self.rng_callback {
            None => unsafe { ffi::hashx_rng_callback(self.ctx, None, null_mut()) },
            Some(callback) => unsafe {
                ffi::hashx_rng_callback(
                    self.ctx,
                    Some(wrapper),
                    callback as *mut RngCallback as *mut c_void,
                );
            },
        }
        unsafe extern "C" fn wrapper(buffer: *mut u64, callback: *mut c_void) {
            let callback = &mut *(callback as *mut RngCallback);
            buffer.write(callback(buffer.read()));
        }
        result
    }
}

impl Drop for HashX {
    fn drop(&mut self) {
        let ctx = mem::replace(&mut self.ctx, null_mut());
        unsafe {
            ffi::hashx_free(ctx);
        }
    }
}

/// Option flags for [`EquiX::new()`]
pub type EquiXFlags = ffi::equix_ctx_flags;

/// A single Equi-X solution
pub type EquiXSolution = ffi::equix_solution;

/// Flags with additional information about solutions
pub type EquiXSolutionFlags = ffi::equix_solution_flags;

/// A buffer with space for several Equi-X solutions
pub type EquiXSolutionsBuffer = ffi::equix_solutions_buffer;

/// Number of indices in a single Equi-X solution
pub const EQUIX_NUM_IDX: usize = ffi::EQUIX_NUM_IDX as usize;

/// Maximum number of Equi-X solutions we will return at once
pub const EQUIX_MAX_SOLS: usize = ffi::EQUIX_MAX_SOLS as usize;

impl Default for EquiXSolutionsBuffer {
    fn default() -> Self {
        Self {
            count: 0,
            flags: ffi::equix_solution_flags(0),
            sols: [EquiXSolution {
                idx: [0; EQUIX_NUM_IDX],
            }; EQUIX_MAX_SOLS],
        }
    }
}

/// Result codes for Equi-X
pub type EquiXResult = ffi::equix_result;

/// Safe wrapper around an Equi-X context
pub struct EquiX(*mut ffi::equix_ctx);

impl EquiX {
    /// Allocate a new Equi-X context
    pub fn new(flags: EquiXFlags) -> Self {
        let ctx = unsafe { ffi::equix_alloc(flags) };
        if ctx.is_null() {
            panic!("out of memory in equix_alloc");
        }
        Self(ctx)
    }

    /// Verify an Equi-X solution against a particular challenge
    #[inline(always)]
    pub fn verify(&mut self, challenge: &[u8], solution: &EquiXSolution) -> EquiXResult {
        unsafe {
            ffi::equix_verify(
                self.0,
                challenge.as_ptr() as *const c_void,
                challenge.len(),
                solution as *const ffi::equix_solution,
            )
        }
    }

    /// Run the solver, returning a variable number of solutions for a challenge
    #[inline(always)]
    pub fn solve(&mut self, challenge: &[u8], buffer: &mut EquiXSolutionsBuffer) -> EquiXResult {
        unsafe {
            ffi::equix_solve(
                self.0,
                challenge.as_ptr() as *const c_void,
                challenge.len(),
                buffer as *mut ffi::equix_solutions_buffer,
            )
        }
    }
}

impl Drop for EquiX {
    fn drop(&mut self) {
        let ctx = mem::replace(&mut self.0, null_mut());
        unsafe {
            ffi::equix_free(ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use hex_literal::hex;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn equix_context() {
        let _ = EquiX::new(EquiXFlags::EQUIX_CTX_TRY_COMPILE | EquiXFlags::EQUIX_CTX_SOLVE);
        let _ = EquiX::new(EquiXFlags::EQUIX_CTX_SOLVE);
        let _ = EquiX::new(EquiXFlags::EQUIX_CTX_VERIFY);
    }

    #[test]
    fn equix_verify_only() {
        let mut ctx = EquiX::new(EquiXFlags::EQUIX_CTX_TRY_COMPILE | EquiXFlags::EQUIX_CTX_VERIFY);

        assert_eq!(
            ctx.verify(
                b"a",
                &EquiXSolution {
                    idx: [0x2227, 0xa173, 0x365a, 0xb47d, 0x1bb2, 0xa077, 0x0d5e, 0xf25f]
                }
            ),
            EquiXResult::EQUIX_OK
        );
        assert_eq!(
            ctx.verify(
                b"a",
                &EquiXSolution {
                    idx: [0x1bb2, 0xa077, 0x0d5e, 0xf25f, 0x2220, 0xa173, 0x365a, 0xb47d]
                }
            ),
            EquiXResult::EQUIX_FAIL_ORDER
        );
        assert_eq!(
            ctx.verify(
                b"a",
                &EquiXSolution {
                    idx: [0x2220, 0xa173, 0x365a, 0xb47d, 0x1bb2, 0xa077, 0x0d5e, 0xf25f]
                }
            ),
            EquiXResult::EQUIX_FAIL_PARTIAL_SUM
        );
    }

    #[test]
    fn equix_solve_only() {
        let mut ctx = EquiX::new(EquiXFlags::EQUIX_CTX_TRY_COMPILE | EquiXFlags::EQUIX_CTX_SOLVE);
        let mut buffer = Default::default();
        assert_eq!(
            ctx.solve(b"01234567890123456789", &mut buffer),
            EquiXResult::EQUIX_OK
        );
        assert_eq!(buffer.count, 5);
        assert_eq!(
            buffer.sols[0].idx,
            [0x4803, 0x6775, 0xc5c9, 0xd1b0, 0x1bc3, 0xe4f6, 0x4027, 0xf5ad,]
        );
        assert_eq!(
            buffer.sols[1].idx,
            [0x5a8a, 0x9542, 0xef99, 0xf0b9, 0x4905, 0x4e29, 0x2da5, 0xfbd5,]
        );
        assert_eq!(
            buffer.sols[2].idx,
            [0x4c79, 0xc935, 0x2bcb, 0xcd0f, 0x0362, 0x9fa9, 0xa62e, 0xf83a,]
        );
        assert_eq!(
            buffer.sols[3].idx,
            [0x5878, 0x6edf, 0x1e00, 0xf5e3, 0x43de, 0x9212, 0xd01e, 0xfd11,]
        );
        assert_eq!(
            buffer.sols[4].idx,
            [0x0b69, 0x2d17, 0x01be, 0x6cb4, 0x0fba, 0x4a9e, 0x8d75, 0xa50f,]
        );
    }

    #[test]
    fn hashx_context() {
        // Context creation should always succeed
        let _ = HashX::new(HashXType::HASHX_TYPE_INTERPRETED);
        let _ = HashX::new(HashXType::HASHX_TYPE_COMPILED);
        let _ = HashX::new(HashXType::HASHX_TRY_COMPILE);
    }

    #[test]
    fn bad_seeds() {
        // Some seed values we expect to fail (and one control).
        // Also tests query_type while we're here.
        let mut ctx = HashX::new(HashXType::HASHX_TYPE_INTERPRETED);
        assert_eq!(ctx.query_type(), Err(HashXResult::HASHX_FAIL_UNPREPARED));
        assert_eq!(ctx.make(b"qfjsfv"), HashXResult::HASHX_FAIL_SEED);
        assert_eq!(ctx.query_type(), Err(HashXResult::HASHX_FAIL_UNPREPARED));
        assert_eq!(ctx.make(b"llompmb"), HashXResult::HASHX_OK);
        assert_eq!(ctx.query_type(), Ok(HashXType::HASHX_TYPE_INTERPRETED));
        assert_eq!(ctx.make(b"mhelht"), HashXResult::HASHX_FAIL_SEED);
        assert_eq!(ctx.query_type(), Err(HashXResult::HASHX_FAIL_UNPREPARED));
    }

    #[test]
    fn hash_values() {
        // Some sample hash values
        let mut ctx = HashX::new(HashXType::HASHX_TRY_COMPILE);
        assert_eq!(ctx.make(b"ebrazua"), HashXResult::HASHX_OK);
        assert_eq!(ctx.exec(0xebc19ba9cafb0863), Ok(hex!("41cb0b4b24551d26")));
        assert_eq!(ctx.make(b"This is a test\0"), HashXResult::HASHX_OK);
        assert_eq!(ctx.exec(0), Ok(hex!("2b2f54567dcbea98")));
        assert_eq!(ctx.exec(123456), Ok(hex!("aebdd50aa67c93af")));
        assert_eq!(
            ctx.make(b"Lorem ipsum dolor sit amet\0"),
            HashXResult::HASHX_OK
        );
        assert_eq!(ctx.exec(123456), Ok(hex!("ab3d155bf4bbb0aa")));
        assert_eq!(ctx.exec(987654321123456789), Ok(hex!("8dfef0497c323274")));
    }

    #[test]
    fn rng_callback_read() {
        // Use a Rng callback to read the sequence of pseudorandom numbers
        // without changing them, and spot check the list we get back.
        let mut ctx = HashX::new(HashXType::HASHX_TRY_COMPILE);
        let seq = Rc::new(RefCell::new(Vec::new()));
        {
            let seq = seq.clone();
            ctx.rng_callback(Some(Box::new(move |value| {
                seq.borrow_mut().push(value);
                value
            })));
        }
        assert_eq!(seq.borrow().len(), 0);
        assert_eq!(ctx.make(b"abc"), HashXResult::HASHX_OK);
        assert_eq!(ctx.exec(12345).unwrap(), hex!("c0bc95da7cc30f37"));
        assert_eq!(seq.borrow().len(), 563);
        assert_eq!(
            seq.borrow()[..4],
            [
                0xf695edd02205449d,
                0x51c1ac51cd19a7d1,
                0xadf4cb303b9814cf,
                0x79793a52d965083d
            ]
        );
    }

    #[test]
    fn rng_callback_replace() {
        // Use a Rng callback to replace the random number stream.
        // We have to choose the replacement somewhat carefully since
        // many stationary replacement values will cause infinite loops.
        let mut ctx = HashX::new(HashXType::HASHX_TYPE_INTERPRETED);
        let counter = Rc::new(RefCell::new(0u32));
        {
            let counter = counter.clone();
            ctx.rng_callback(Some(Box::new(move |_value| {
                *counter.borrow_mut() += 1;
                0x0807060504030201
            })));
        }
        assert_eq!(*counter.borrow(), 0);
        assert_eq!(ctx.make(b"abc"), HashXResult::HASHX_OK);
        assert_eq!(ctx.exec(12345).unwrap(), hex!("825a9b6dd5d074af"));
        assert_eq!(*counter.borrow(), 575);
    }

    #[test]
    fn rng_large_compiler_output() {
        // This is really a general HashX test rather than a test for the Rust
        // wrapper. It's easier to implement here than in hashx-test, since
        // the Rng callback is disabled by default in the cmake build system.
        //
        // The purpose of this test is to use a specially crafted Rng sequence
        // to generate an especially large compiled hash program, to test for
        // length-related problems.
        //
        // There are various ways to generate these Rng sequences. The fuzzer
        // in Arti will do this on its own. The sequence here was found with
        // a simple ad-hoc optimizer that modifies one byte at a time in a
        // search for peaks in either x86_64 or aarch64 code size.
        //
        // The expected compiled program length:
        //
        //  - On x86_64, 3188 bytes
        //    (safely less than a page still)
        //
        //  - On aarch64, 4388 bytes
        //    (would overflow a single page buffer by 292 bytes)
        //

        const SEQUENCE: [u64; 558] = [
            0xffffffffffffffff, // 0
            0xffffffffffffffff, // 1
            0xfbfefefbfbfeffff, // 2
            0xffffffffffffffff, // 3
            0xffffffffffffffff, // 4
            0xfffffffffffffdff, // 5
            0xffffffffffffffff, // 6
            0xffffffffffffffff, // 7
            0xfffffffefffffffd, // 8
            0xffffffffffffffff, // 9
            0xffffffffffffffff, // 10
            0xffffffffffffffff, // 11
            0xfefffffeffffffff, // 12
            0xffffffffffffffff, // 13
            0xffffffffffffffff, // 14
            0xfefbfcfdfefffefb, // 15
            0xfffffffffffffffc, // 16
            0xffffffffffffffff, // 17
            0xffffffffffffffff, // 18
            0xffffffffffffffff, // 19
            0xffffffffffffffff, // 20
            0xfffffffffffefffe, // 21
            0xfffffefffbfefbfe, // 22
            0xffffffffffffffff, // 23
            0xfefeffffffffffff, // 24
            0xffffffffffffffff, // 25
            0xffffffffffffffff, // 26
            0xffffffffffffffff, // 27
            0xffffffffffffffff, // 28
            0xffffffffffffffff, // 29
            0xffffffffffffffff, // 30
            0xffffffffffffffff, // 31
            0xffffffffffffffff, // 32
            0xffffffffffffffff, // 33
            0xfffffffeffffffff, // 34
            0xffffffffffffffff, // 35
            0xfffffffffffffffe, // 36
            0xffffffffffffffff, // 37
            0xfbfbfffefffbffff, // 38
            0xffffffffffffffff, // 39
            0xfffffffffffffffe, // 40
            0xffffffffffffffff, // 41
            0xffffffffffffffff, // 42
            0xffffffffffffffff, // 43
            0xffffffffffffffff, // 44
            0xfffffffeffffffff, // 45
            0xffffffffffffffff, // 46
            0xffffffffffffffff, // 47
            0xffffffffffffffff, // 48
            0xfefefffdffffffff, // 49
            0xfefbfefefefcfdff, // 50
            0xffffffffffffffff, // 51
            0xffffffffffffffff, // 52
            0xffffffffffffffff, // 53
            0xffffffffffffffff, // 54
            0xfefffffffefefffc, // 55
            0xfffffffeffffffff, // 56
            0xfbfefffefbfefefb, // 57
            0xfffffffeffffffff, // 58
            0xffffffffffffffff, // 59
            0xfffffffffffffefc, // 60
            0xfffffffffffffffc, // 61
            0xffffffffffffffff, // 62
            0xffffffffffffffff, // 63
            0xffffffffffffffff, // 64
            0xfffffefdffffffff, // 65
            0xffffffffffffffff, // 66
            0xffffffffffffffff, // 67
            0xffffffffffffffff, // 68
            0xfefbfefbfefbfbfe, // 69
            0xffffffffffffffff, // 70
            0xffffffffffffffff, // 71
            0xfffefeffffffffff, // 72
            0xfffffffffffffffe, // 73
            0xffffffffffffffff, // 74
            0xffffffffffffffff, // 75
            0xfeffffffffffffff, // 76
            0xffffffffffffffff, // 77
            0xffffffffffffffff, // 78
            0xffffffffffffffff, // 79
            0xffffffffffffffff, // 80
            0xffffffffffffffff, // 81
            0xfffffffefcfdfeff, // 82
            0xffffffffffffffff, // 83
            0xfefeffffffffffff, // 84
            0xffffffffffffffff, // 85
            0xffffffffffffffff, // 86
            0xffffffffffffffff, // 87
            0xfffffffdffffffff, // 88
            0xffffffffffffffff, // 89
            0xffffffffffffffff, // 90
            0xffffffffffffffff, // 91
            0xfefbfffefefbfbfe, // 92
            0xffffffffffffffff, // 93
            0xfffffffeffffffff, // 94
            0xfffffffffefeffff, // 95
            0xffffffffffffffff, // 96
            0xfffffffffffffffe, // 97
            0xffffffffffffffff, // 98
            0xffffffffffffffff, // 99
            0xffffffffffffffff, // 100
            0xfffffffffffffffe, // 101
            0xfffffffffeffffff, // 102
            0xfdfdffffffffffff, // 103
            0xfbfefbfefefefefe, // 104
            0xffffffffffffffff, // 105
            0xffffffffffffffff, // 106
            0xfffffffffffffffd, // 107
            0xfefffffffffefdff, // 108
            0xfffffffffefffffe, // 109
            0xfffffffffffffffe, // 110
            0xffffffffffffffff, // 111
            0xffffffffffffffff, // 112
            0xfbfefef8fffefefb, // 113
            0xfffffffcffffffff, // 114
            0xfefefefdffffffff, // 115
            0xffffffffffffffff, // 116
            0xfffffffdffffffff, // 117
            0xfffffffffdfdfdfb, // 118
            0xffffffffffffffff, // 119
            0xfffdfdffffffffff, // 120
            0xffffffffffffffff, // 121
            0xffffffffffffffff, // 122
            0xfffffffffffffffd, // 123
            0xfdfffefffffcfffe, // 124
            0xfcfefffffffefeff, // 125
            0xffffffffffffffff, // 126
            0xffffffffffffffff, // 127
            0xffffffffffffffff, // 128
            0xfffbf8f8fbf8fefe, // 129
            0xfffffffffefcfdff, // 130
            0xfffffffffffffffd, // 131
            0xffffffffffffffff, // 132
            0xfffffffffcfcffff, // 133
            0xffffffffffffffff, // 134
            0xffffffffffffffff, // 135
            0xfffffffffdfefdff, // 136
            0xffffffffffffffff, // 137
            0xfcfefbfdfffffffe, // 138
            0xfffffffffeffffff, // 139
            0xf8fbfefefefffeff, // 140
            0xffffffffffffffff, // 141
            0xfefefefffefffffe, // 142
            0xffffffffffffffff, // 143
            0xfffffffffcfefeff, // 144
            0xffffffffffffffff, // 145
            0xfffffffffffffffe, // 146
            0xfffffffffffffffe, // 147
            0xffffffffffffffff, // 148
            0xfffffffffefffeff, // 149
            0xfffefffeffffffff, // 150
            0xffffffffffffffff, // 151
            0xffffffffffffffff, // 152
            0xfffffbfefffffcff, // 153
            0xffffffffffffffff, // 154
            0xfdfefefaffffffff, // 155
            0xffffffffffffffff, // 156
            0xfffffffffffffffd, // 157
            0xfffffffffffffffe, // 158
            0xffffffffffffffff, // 159
            0xffffffffffffffff, // 160
            0xfdfefefbfffbfffe, // 161
            0xfffffffefffffffe, // 162
            0xffffffffffffffff, // 163
            0xffffffffffffffff, // 164
            0xfeffffffffffffff, // 165
            0xfffdfffdffffffff, // 166
            0xfffffffdffffffff, // 167
            0xfeffffffffffffff, // 168
            0xffffffffffffffff, // 169
            0xffffffffffffffff, // 170
            0xffffffffffffffff, // 171
            0xfcfffefefffefbfe, // 172
            0xffffffffffffffff, // 173
            0xfffffffffffeffff, // 174
            0xffffffffffffffff, // 175
            0xfffffffffffffffe, // 176
            0xfffffffffdfefdfd, // 177
            0xffffffffffffffff, // 178
            0xffffffffffffffff, // 179
            0xfffffffdffffffff, // 180
            0xffffffffffffffff, // 181
            0xffffffffffffffff, // 182
            0xffffffffffffffff, // 183
            0xffffffffffffffff, // 184
            0xfbfffefffefefbfd, // 185
            0xfffffffffffeffff, // 186
            0xffffffffffffffff, // 187
            0xffffffffffffffff, // 188
            0xffffffffffffffff, // 189
            0xffffffffffffffff, // 190
            0xfffdfeffffffffff, // 191
            0xffffffffffffffff, // 192
            0xfffffffeffffffff, // 193
            0xffffffffffffffff, // 194
            0xffffffffffffffff, // 195
            0xfffffffefeffffff, // 196
            0xfcfefff8fefffbfe, // 197
            0xffffffffffffffff, // 198
            0xffffffffffffffff, // 199
            0xffffffffffffffff, // 200
            0xffffffffffffffff, // 201
            0xffffffffffffffff, // 202
            0xffffffffffffffff, // 203
            0xffffffffffffffff, // 204
            0xfbfbfefbfefefeff, // 205
            0xffffffffffffffff, // 206
            0xfffeffffffffffff, // 207
            0xffffffffffffffff, // 208
            0xffffffffffffffff, // 209
            0xffffffffffffffff, // 210
            0xffffffffffffffff, // 211
            0xffffffffffffffff, // 212
            0xfffffffffefeffff, // 213
            0xfefefefeffffffff, // 214
            0xffffffffffffffff, // 215
            0xffffffffffffffff, // 216
            0xfffffffffefeffff, // 217
            0xfbfefbfefffefefb, // 218
            0xffffffffffffffff, // 219
            0xfffffffffffffffe, // 220
            0xfffffffefdfffefe, // 221
            0xffffffffffffffff, // 222
            0xffffffffffffffff, // 223
            0xffffffffffffffff, // 224
            0xfffefffcffffffff, // 225
            0xfffffefffffdfdff, // 226
            0xfffefeffffffffff, // 227
            0xfffffeffffffffff, // 228
            0xffffffffffffffff, // 229
            0xfffffffffefefefd, // 230
            0xfcfdfefffefffffe, // 231
            0xfefdffffffffffff, // 232
            0xfffffffeffffffff, // 233
            0xfdfefdffffffffff, // 234
            0xffffffffffffffff, // 235
            0xfdfefffeffffffff, // 236
            0xffffffffffffffff, // 237
            0xffffffffffffffff, // 238
            0xfbfffffefbfefefe, // 239
            0xfefcfdffffffffff, // 240
            0xfffffffffffffffe, // 241
            0xfffffffefefdfefd, // 242
            0xffffffffffffffff, // 243
            0xfffeffffffffffff, // 244
            0xffffffffffffffff, // 245
            0xfffffffeffffffff, // 246
            0xffffffffffffffff, // 247
            0xfffffffffefefeff, // 248
            0xfffffffdfefffefe, // 249
            0xfffefeffffffffff, // 250
            0xffffffffffffffff, // 251
            0xfbfbfefefefbfffe, // 252
            0xffffffffffffffff, // 253
            0xfffffffeffffffff, // 254
            0xfffffffeffffffff, // 255
            0xfefffeffffffffff, // 256
            0xfffffdffffffffff, // 257
            0xffffffffffffffff, // 258
            0xffffffffffffffff, // 259
            0xfffffffffdfffdff, // 260
            0xfffffffffefffffe, // 261
            0xfefffffffffffefe, // 262
            0xfefffcfdfffefefb, // 263
            0xffffffffffffffff, // 264
            0xffffffffffffffff, // 265
            0xffffffffffffffff, // 266
            0xfffffffffeffffff, // 267
            0xffffffffffffffff, // 268
            0xffffffffffffffff, // 269
            0xffffffffffffffff, // 270
            0xfefbfefbfbfefefe, // 271
            0xfffffffffffffdff, // 272
            0xfffffffffffffffe, // 273
            0xffffffffffffffff, // 274
            0xffffffffffffffff, // 275
            0xffffffffffffffff, // 276
            0xffffffffffffffff, // 277
            0xffffffffffffffff, // 278
            0xffffffffffffffff, // 279
            0xfffffcfcfffffeff, // 280
            0xffffffffffffffff, // 281
            0xfbf8fefefbfbfeff, // 282
            0xfffffffffffffffe, // 283
            0xfffffffffffffffe, // 284
            0xffffffffffffffff, // 285
            0xffffffffffffffff, // 286
            0xffffffffffffffff, // 287
            0xffffffffffffffff, // 288
            0xffffffffffffffff, // 289
            0xffffffffffffffff, // 290
            0xffffffffffffffff, // 291
            0xffffffffffffffff, // 292
            0xffffffffffffffff, // 293
            0xffffffffffffffff, // 294
            0xffffffffffffffff, // 295
            0xfefefdfcfdfefffe, // 296
            0xfffffffeffffffff, // 297
            0xffffffffffffffff, // 298
            0xfffffeffffffffff, // 299
            0xffffffffffffffff, // 300
            0xfffefffffefefffe, // 301
            0xfffffffeffffffff, // 302
            0xffffffffffffffff, // 303
            0xfbfffefefbfefffe, // 304
            0xffffffffffffffff, // 305
            0xfffffffffffeffff, // 306
            0xffffffffffffffff, // 307
            0xfffeffffffffffff, // 308
            0xffffffffffffffff, // 309
            0xffffffffffffffff, // 310
            0xffffffffffffffff, // 311
            0xffffffffffffffff, // 312
            0xffffffffffffffff, // 313
            0xffffffffffffffff, // 314
            0xffffffffffffffff, // 315
            0xfffffffeffffffff, // 316
            0xfbfefbfbfefbfeff, // 317
            0xffffffffffffffff, // 318
            0xfffffffefefeffff, // 319
            0xfffffffeffffffff, // 320
            0xffffffffffffffff, // 321
            0xffffffffffffffff, // 322
            0xffffffffffffffff, // 323
            0xffffffffffffffff, // 324
            0xffffffffffffffff, // 325
            0xffffffffffffffff, // 326
            0xffffffffffffffff, // 327
            0xffffffffffffffff, // 328
            0xfffffffffefefeff, // 329
            0xfefefefefbfdfeff, // 330
            0xffffffffffffffff, // 331
            0xffffffffffffffff, // 332
            0xfffffffffeffffff, // 333
            0xffffffffffffffff, // 334
            0xfefffffffffffffe, // 335
            0xfcfbfefffefbfefe, // 336
            0xfffffffffffefeff, // 337
            0xffffffffffffffff, // 338
            0xffffffffffffffff, // 339
            0xfeffffffffffffff, // 340
            0xfffdfeffffffffff, // 341
            0xffffffffffffffff, // 342
            0xffffffffffffffff, // 343
            0xffffffffffffffff, // 344
            0xffffffffffffffff, // 345
            0xfffffffdffffffff, // 346
            0xffffffffffffffff, // 347
            0xfefbfbfefbfeffff, // 348
            0xffffffffffffffff, // 349
            0xffffffffffffffff, // 350
            0xffffffffffffffff, // 351
            0xffffffffffffffff, // 352
            0xffffffffffffffff, // 353
            0xffffffffffffffff, // 354
            0xffffffffffffffff, // 355
            0xfffffffeffffffff, // 356
            0xffffffffffffffff, // 357
            0xffffffffffffffff, // 358
            0xfefbfefffefffbff, // 359
            0xffffffffffffffff, // 360
            0xfefffffffffffffe, // 361
            0xffffffffffffffff, // 362
            0xffffffffffffffff, // 363
            0xffffffffffffffff, // 364
            0xfffffefdffffffff, // 365
            0xfffffffeffffffff, // 366
            0xffffffffffffffff, // 367
            0xfffefefefffffffe, // 368
            0xfffffffffffffffe, // 369
            0xfffffffffffffffc, // 370
            0xfcfdfffefefbfffe, // 371
            0xfcfdfcfcfffffffe, // 372
            0xffffffffffffffff, // 373
            0xffffffffffffffff, // 374
            0xffffffffffffffff, // 375
            0xfdfdfffeffffffff, // 376
            0xfffffffffffffeff, // 377
            0xfffffffeffffffff, // 378
            0xfbfefefbfbfefefb, // 379
            0xfffffffdffffffff, // 380
            0xffffffffffffffff, // 381
            0xffffffffffffffff, // 382
            0xffffffffffffffff, // 383
            0xffffffffffffffff, // 384
            0xffffffffffffffff, // 385
            0xffffffffffffffff, // 386
            0xfffffffffffffffe, // 387
            0xfffffffffffffffe, // 388
            0xffffffffffffffff, // 389
            0xffffffffffffffff, // 390
            0xffffffffffffffff, // 391
            0xfefefbfbfefffeff, // 392
            0xfffffffffffffffe, // 393
            0xffffffffffffffff, // 394
            0xfffffffffffffffd, // 395
            0xffffffffffffffff, // 396
            0xffffffffffffffff, // 397
            0xffffffffffffffff, // 398
            0xfffeffffffffffff, // 399
            0xffffffffffffffff, // 400
            0xffffffffffffffff, // 401
            0xfffffefeffffffff, // 402
            0xfefdfcfefffffeff, // 403
            0xffffffffffffffff, // 404
            0xfffffffffffffffe, // 405
            0xffffffffffffffff, // 406
            0xffffffffffffffff, // 407
            0xfffffffeffffffff, // 408
            0xffffffffffffffff, // 409
            0xfffffffffefeffff, // 410
            0xfefefbfbfefbfefe, // 411
            0xfffffffffffefffe, // 412
            0xffffffffffffffff, // 413
            0xffffffffffffffff, // 414
            0xfffffffffffffffe, // 415
            0xffffffffffffffff, // 416
            0xffffffffffffffff, // 417
            0xfffffffffffffffe, // 418
            0xfffffffffffffffe, // 419
            0xfffffffffffffffe, // 420
            0xffffffffffffffff, // 421
            0xfffffffefffeffff, // 422
            0xfffffffeffffffff, // 423
            0xfffffffeffffffff, // 424
            0xfefefefefefbfbfe, // 425
            0xfffffffffdfffefb, // 426
            0xfffffffeffffffff, // 427
            0xfffffffeffffffff, // 428
            0xfffdfdfffffffffe, // 429
            0xfef8fffbfefbfeff, // 430
            0xffffffffffffffff, // 431
            0xffffffffffffffff, // 432
            0xfffffffffffefdfe, // 433
            0xffffffffffffffff, // 434
            0xffffffffffffffff, // 435
            0xffffffffffffffff, // 436
            0xffffffffffffffff, // 437
            0xfefffeffffffffff, // 438
            0xfcfdfefbfffefefb, // 439
            0xffffffffffffffff, // 440
            0xffffffffffffffff, // 441
            0xffffffffffffffff, // 442
            0xffffffffffffffff, // 443
            0xfffefeffffffffff, // 444
            0xffffffffffffffff, // 445
            0xffffffffffffffff, // 446
            0xfffffffeffffffff, // 447
            0xffffffffffffffff, // 448
            0xffffffffffffffff, // 449
            0xfefbfbfefffffffe, // 450
            0xffffffffffffffff, // 451
            0xfffffffffeffffff, // 452
            0xffffffffffffffff, // 453
            0xffffffffffffffff, // 454
            0xfffffffeffffffff, // 455
            0xffffffffffffffff, // 456
            0xffffffffffffffff, // 457
            0xffffffffffffffff, // 458
            0xffffffffffffffff, // 459
            0xfffffffefffffffe, // 460
            0xfbfefefbfffbfbfe, // 461
            0xfffffffffffffffe, // 462
            0xffffffffffffffff, // 463
            0xfefdfeffffffffff, // 464
            0xffffffffffffffff, // 465
            0xffffffffffffffff, // 466
            0xffffffffffffffff, // 467
            0xfefffefeffffffff, // 468
            0xfffffffffeffffff, // 469
            0xffffffffffffffff, // 470
            0xfffffffdffffffff, // 471
            0xffffffffffffffff, // 472
            0xfffffffffdfbfbfe, // 473
            0xfcfdfefffefbfffe, // 474
            0xfffffffffffdfffe, // 475
            0xfffffffffefdffff, // 476
            0xffffffffffffffff, // 477
            0xfefffffeffffffff, // 478
            0xfdfffefdfefffefd, // 479
            0xffffffffffffffff, // 480
            0xfffbfefbfefbfefb, // 481
            0xfbfcfdfdffffffff, // 482
            0xfffffffffffffffe, // 483
            0xfffffffffffffffe, // 484
            0xffffffffffffffff, // 485
            0xfffffffffffffffe, // 486
            0xfffffefffffffffe, // 487
            0xffffffffffffffff, // 488
            0xffffffffffffffff, // 489
            0xffffffffffffffff, // 490
            0xffffffffffffffff, // 491
            0xffffffffffffffff, // 492
            0xffffffffffffffff, // 493
            0xffffffffffffffff, // 494
            0xfbfefefbfffef8fe, // 495
            0xffffffffffffffff, // 496
            0xffffffffffffffff, // 497
            0xffffffffffffffff, // 498
            0xffffffffffffffff, // 499
            0xfffffffeffffffff, // 500
            0xffffffffffffffff, // 501
            0xfffffffffffffffe, // 502
            0xffffffffffffffff, // 503
            0xfffffffffffffffe, // 504
            0xffffffffffffffff, // 505
            0xfffffffffffffffe, // 506
            0xfcfdfffffefefbff, // 507
            0xffffffffffffffff, // 508
            0xffffffffffffffff, // 509
            0xffffffffffffffff, // 510
            0xffffffffffffffff, // 511
            0xffffffffffffffff, // 512
            0xfefbfefefefefbfe, // 513
            0xffffffffffffffff, // 514
            0xfffffeffffffffff, // 515
            0xffffffffffffffff, // 516
            0xfffffffeffffffff, // 517
            0xfffffffeffffffff, // 518
            0xfffffffeffffffff, // 519
            0xfffffffefefeffff, // 520
            0xffffffffffffffff, // 521
            0xfefbfbfefbfefefb, // 522
            0xffffffffffffffff, // 523
            0xffffffffffffffff, // 524
            0xffffffffffffffff, // 525
            0xffffffffffffffff, // 526
            0xffffffffffffffff, // 527
            0xffffffffffffffff, // 528
            0xffffffffffffffff, // 529
            0xffffffffffffffff, // 530
            0xffffffffffffffff, // 531
            0xffffffffffffffff, // 532
            0xffffffffffffffff, // 533
            0xfffefefbfcfdfeff, // 534
            0xffffffffffffffff, // 535
            0xffffffffffffffff, // 536
            0xffffffffffffffff, // 537
            0xffffffffffffffff, // 538
            0xffffffffffffffff, // 539
            0xffffffffffffffff, // 540
            0xffffffffffffffff, // 541
            0xffffffffffffffff, // 542
            0xfbfbfefffefefbfb, // 543
            0xffffffffffffffff, // 544
            0xffffffffffffffff, // 545
            0xffffffffffffffff, // 546
            0xffffffffffffffff, // 547
            0xffffffffffffffff, // 548
            0xffffffffffffffff, // 549
            0xffffffffffffffff, // 550
            0xffffffffffffffff, // 551
            0xffffffffffffffff, // 552
            0xfefefbffffffffff, // 553
            0xffffffffffffffff, // 554
            0xffffffffffffffff, // 555
            0xffffffffffffffff, // 556
            0xffffffffffffffff, // 557
        ];

        // Do a test run against the interpreter, then check the compiler.
        for hash_type in [
            HashXType::HASHX_TYPE_INTERPRETED,
            HashXType::HASHX_TYPE_COMPILED,
        ] {
            let mut ctx = HashX::new(hash_type);

            // Fully replace the Rng stream, which must be exactly the right size
            let counter = Rc::new(RefCell::new(0_usize));
            {
                let counter = counter.clone();
                ctx.rng_callback(Some(Box::new(move |_value| {
                    let mut counter = counter.borrow_mut();
                    let result = SEQUENCE[*counter];
                    *counter += 1;
                    result
                })));
            }

            // Seed choice: This seed will normally fail constraint checks.
            // Using it here is a way of verifying that Rng replacement works.
            assert_eq!(*counter.borrow(), 0);
            assert_eq!(ctx.make(b"qfjsfv"), HashXResult::HASHX_OK);
            assert_eq!(*counter.borrow(), SEQUENCE.len());
            assert_eq!(ctx.query_type(), Ok(hash_type));

            // Make sure we can run the hash function, spot-testing the output.
            assert_eq!(ctx.exec(0).unwrap(), hex!("7d7442b95fc9ea3d"));
            assert_eq!(ctx.exec(123).unwrap(), hex!("1519ee923bf1e699"));
            assert_eq!(ctx.exec(12345).unwrap(), hex!("726c4073ff1bb595"));
        }
    }
}
