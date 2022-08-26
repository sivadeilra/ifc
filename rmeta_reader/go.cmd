@echo off

rustc --crate-name hello --crate-type rlib --out-dir out inputs\hello.rs
if errorlevel 1 exit /b 1

cargo test -- --nocapture
