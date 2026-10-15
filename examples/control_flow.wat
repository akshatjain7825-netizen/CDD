(module
  (type (;0;) (func (param i32) (result i32)))
  (memory (export "memory") 18)
  (global (;0;) (mut i32) (i32.const 0))
  (func $sum_up_to (type 0) (param i32) (result i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    block
    i32.const 0
    local.set 1
    i32.const 1
    local.tee 2
    local.set 3
    local.get 0
    local.set 4
    local.get 1
    local.set 6
    end
    loop
    block
    block
    local.get 3
    local.get 4
    i32.le_s
    local.tee 5
    br_if 1
    end
    local.get 6
    return
    end
    local.get 6
    local.get 3
    i32.add
    local.set 7
    i32.const 1
    local.set 8
    local.get 3
    local.get 8
    i32.add
    local.tee 9
    local.set 3
    local.get 4
    local.set 4
    local.get 7
    local.set 6
    br 0
    end
    unreachable
    end
  )
  (func $count_even (type 0) (param i32) (result i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    block
    i32.const 0
    local.set 1
    i32.const 0
    local.tee 2
    local.set 3
    local.get 0
    local.set 4
    local.get 1
    local.set 10
    end
    loop
    block
    block
    local.get 3
    local.get 4
    i32.lt_s
    local.tee 5
    br_if 1
    end
    local.get 10
    return
    end
    block
    block
    i32.const 2
    local.set 6
    local.get 3
    local.get 6
    i32.rem_s
    local.set 7
    i32.const 0
    local.set 8
    local.get 7
    local.get 8
    i32.eq
    local.set 9
    local.get 3
    local.set 13
    local.get 4
    local.set 16
    local.get 10
    local.set 17
    local.get 9
    br_if 1
    end
    i32.const 1
    local.set 11
    local.get 10
    local.get 11
    i32.add
    local.set 12
    local.get 3
    local.set 13
    local.get 4
    local.set 16
    local.get 12
    local.set 17
    end
    i32.const 1
    local.set 14
    local.get 13
    local.get 14
    i32.add
    local.tee 15
    local.set 3
    local.get 16
    local.set 4
    local.get 17
    local.set 10
    br 0
    end
    unreachable
    end
  )
  (export "sum_up_to" (func 0))
  (export "count_even" (func 1))
  (export "memory" (memory 0))
)
