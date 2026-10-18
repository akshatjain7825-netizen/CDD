(module
  (type (;0;) (func (param i32 i32) (result i32)))
  (memory (export "memory") 18)
  (global (;0;) (mut i32) (i32.const 0))
  (func $stack_math (type 0) (param i32) (param i32) (result i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    global.get 0
    i32.const 16
    i32.sub
    local.tee 2
    global.set 0
    local.get 2
    i32.const 1024
    i32.lt_u
    if
    unreachable
    end
    local.get 2
    local.set 3
    i32.const 42
    local.set 4
    local.get 0
    local.get 4
    i32.add
    local.set 5
    local.get 3
    local.get 5
    i32.store offset=0
    local.get 3
    i32.load offset=0
    local.tee 7
    local.get 1
    i32.add
    local.set 8
    local.get 2
    i32.const 16
    i32.add
    global.set 0
    local.get 8
    return
    end
  )
  (export "stack_math" (func 0))
  (export "memory" (memory 0))
)
