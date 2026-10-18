(module
  (type (;0;) (func (param i32) (result i32)))
  (type (;1;) (func (param i32 i32) (result i32)))
  (memory (export "memory") 18)
  (global (;0;) (mut i32) (i32.const 0))
  (func $square (type 0) (param i32) (result i32)
    (local i32)
    local.get 0
    local.get 0
    i32.mul
    local.tee 1
    return
    end
  )
  (func $sum_of_squares (type 1) (param i32) (param i32) (result i32)
    (local i32)
    (local i32)
    (local i32)
    local.get 0
    call 0
    local.set 2
    local.get 1
    call 0
    local.set 3
    local.get 2
    local.get 3
    i32.add
    local.tee 4
    return
    end
  )
  (func $factorial (type 0) (param i32) (result i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    block
    block
    i32.const 1
    local.set 1
    local.get 0
    local.get 1
    i32.le_s
    local.tee 2
    i32.eqz
    br_if 1
    end
    i32.const 1
    local.tee 3
    return
    end
    i32.const 1
    local.set 4
    local.get 0
    local.get 4
    i32.sub
    local.tee 5
    call 2
    local.set 6
    local.get 0
    local.get 6
    i32.mul
    local.tee 7
    return
    end
  )
  (export "square" (func 0))
  (export "sum_of_squares" (func 1))
  (export "factorial" (func 2))
  (export "memory" (memory 0))
)
