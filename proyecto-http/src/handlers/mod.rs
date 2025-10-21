//! Namespaces para separar la lógica por tipo de endpoint.
//! Se llenarán en sprints siguientes.

pub mod basic;
    // Rutas livianas: /reverse, /toupper, /random, /timestamp, /hash, /help, /sleep, /simulate, /fibonacci, create/deletefile…

pub mod cpu;
    // Rutas CPU-bound reales: /isprime, /factor, /pi, /mandelbrot, /matrixmul…

pub mod io;
    // Rutas IO-bound reales: /sortfile, /wordcount, /grep, /compress, /hashfile…

// Re-exportamos las funciones básicas más usadas
pub use self::basic::{not_found, reverse, status, timestamp, toupper};