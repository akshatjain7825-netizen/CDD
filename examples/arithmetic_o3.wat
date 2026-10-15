(module
  (type (;0;) (func (param i32 i32) (result i32)))
  (type (;1;) (func (param f64 f64) (result f64)))
  (memory (export "memory") 18)
  (global (;0;) (mut i32) (i32.const 0))
  (func $add (type 0) (param i32) (param i32) (result i32)
    (local i32)
    local.get 0
    local.get 1
    i32.add
    local.tee 2
    return
    end
  )
  (func $compute (type 0) (param i32) (param i32) (result i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    i32.const 4
    local.set 2
    local.get 0
    local.get 2
    i32.mul
    local.set 3
    i32.const 2
    local.set 4
    local.get 1
    local.get 4
    i32.div_s
    local.set 5
    local.get 3
    local.get 5
    i32.add
    local.set 6
    i32.const 255
    local.set 7
    local.get 6
    local.get 7
    i32.xor
    local.tee 8
    return
    end
  )
  (func $float_calc (type 1) (param f64) (param f64) (result f64)
    (local f64)
    (local f64)
    (local f64)
    (local f64)
    (local f64)
    f64.const 2.5
    local.set 2
    local.get 0
    local.get 2
    f64.mul
    local.set 3
    f64.const 1.5
    local.set 4
    local.get 1
    local.get 4
    f64.div
    local.set 5
    local.get 3
    local.get 5
    f64.add
    local.tee 6
    return
    end
  )
  (export "add" (func 0))
  (export "compute" (func 1))
  (export "float_calc" (func 2))
  (export "memory" (memory 0))
)
