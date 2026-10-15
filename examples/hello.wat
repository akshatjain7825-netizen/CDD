(module
  (type (;0;) (func (param i32 i32)))
  (type (;1;) (func (result i32)))
  (import "env" "print" (func (;0;) (type 0)))
  (memory (export "memory") 18)
  (global (;0;) (mut i32) (i32.const 0))
  (func $main (type 1) (result i32)
    (local i32)
    (local i32)
    (local i32)
    i32.const 1408
    local.set 0
    i32.const 19
    local.set 1
    local.get 0
    local.get 1
    call 0
    i32.const 0
    local.tee 2
    return
    end
  )
  (export "main" (func 1))
  (export "memory" (memory 0))
)
