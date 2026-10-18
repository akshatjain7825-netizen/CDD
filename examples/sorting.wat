(module
  (type (;0;) (func (param i32 i32)))
  (type (;1;) (func (param i32) (result i32)))
  (type (;2;) (func (param i32 i32) (result i32)))
  (type (;3;) (func (param i32 i32 i32) (result i32)))
  (memory (export "memory") 18)
  (global (;0;) (mut i32) (i32.const 0))
  (func $swap (type 0) (param i32) (param i32)
    (local i32)
    (local i32)
    local.get 0
    i32.load offset=0
    local.set 2
    local.get 1
    i32.load offset=0
    local.set 3
    local.get 0
    local.get 3
    i32.store offset=0
    local.get 1
    local.get 2
    i32.store offset=0
    return
    end
  )
  (func $bubble_sort_count (type 1) (param i32) (result i32)
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
    global.get 0
    i32.const 32
    i32.sub
    local.tee 1
    global.set 0
    local.get 1
    i32.const 1024
    i32.lt_u
    if
    unreachable
    end
    block
    block
    local.get 1
    local.set 2
    i32.const 5
    local.set 3
    local.get 2
    local.get 3
    i32.store offset=0
    local.get 1
    i32.const 4
    i32.add
    local.set 4
    i32.const 3
    local.set 5
    local.get 4
    local.get 5
    i32.store offset=0
    local.get 1
    i32.const 8
    i32.add
    local.set 6
    i32.const 8
    local.set 7
    local.get 6
    local.get 7
    i32.store offset=0
    local.get 1
    i32.const 12
    i32.add
    local.set 8
    i32.const 1
    local.set 9
    local.get 8
    local.get 9
    i32.store offset=0
    local.get 1
    i32.const 16
    i32.add
    local.set 10
    i32.const 9
    local.set 11
    local.get 10
    local.get 11
    i32.store offset=0
    i32.const 0
    local.set 12
    local.get 2
    i32.load offset=0
    local.set 14
    local.get 4
    i32.load offset=4
    local.set 16
    local.get 14
    local.get 16
    i32.gt_s
    local.set 17
    local.get 12
    local.set 20
    local.get 17
    i32.eqz
    br_if 1
    end
    local.get 1
    local.set 18
    local.get 1
    i32.const 4
    i32.add
    local.set 19
    local.get 18
    local.get 19
    call 0
    local.get 12
    local.set 20
    end
    block
    block
    i32.const 1
    local.set 21
    local.get 20
    local.get 21
    i32.add
    local.set 22
    local.get 1
    i32.const 4
    i32.add
    local.tee 23
    i32.load offset=4
    local.set 24
    local.get 1
    i32.const 8
    i32.add
    local.tee 25
    i32.load offset=8
    local.set 26
    local.get 24
    local.get 26
    i32.gt_s
    local.set 27
    local.get 22
    local.set 30
    local.get 27
    i32.eqz
    br_if 1
    end
    local.get 1
    i32.const 4
    i32.add
    local.set 28
    local.get 1
    i32.const 8
    i32.add
    local.set 29
    local.get 28
    local.get 29
    call 0
    local.get 22
    local.set 30
    end
    block
    block
    i32.const 1
    local.set 31
    local.get 30
    local.get 31
    i32.add
    local.set 32
    local.get 1
    i32.const 8
    i32.add
    local.tee 33
    i32.load offset=8
    local.set 34
    local.get 1
    i32.const 12
    i32.add
    local.tee 35
    i32.load offset=12
    local.set 36
    local.get 34
    local.get 36
    i32.gt_s
    local.set 37
    local.get 32
    local.set 40
    local.get 37
    i32.eqz
    br_if 1
    end
    local.get 1
    i32.const 8
    i32.add
    local.set 38
    local.get 1
    i32.const 12
    i32.add
    local.set 39
    local.get 38
    local.get 39
    call 0
    local.get 32
    local.set 40
    end
    block
    block
    i32.const 1
    local.set 41
    local.get 40
    local.get 41
    i32.add
    local.set 42
    local.get 1
    i32.const 12
    i32.add
    local.tee 43
    i32.load offset=12
    local.set 44
    local.get 1
    i32.const 16
    i32.add
    local.tee 45
    i32.load offset=16
    local.set 46
    local.get 44
    local.get 46
    i32.gt_s
    local.set 47
    local.get 42
    local.set 50
    local.get 47
    i32.eqz
    br_if 1
    end
    local.get 1
    i32.const 12
    i32.add
    local.set 48
    local.get 1
    i32.const 16
    i32.add
    local.set 49
    local.get 48
    local.get 49
    call 0
    local.get 42
    local.set 50
    end
    block
    block
    i32.const 1
    local.set 51
    local.get 50
    local.get 51
    i32.add
    local.set 52
    local.get 1
    local.tee 53
    i32.load offset=0
    local.set 54
    local.get 1
    i32.const 4
    i32.add
    local.tee 55
    i32.load offset=4
    local.set 56
    local.get 54
    local.get 56
    i32.gt_s
    local.set 57
    local.get 52
    local.set 60
    local.get 57
    i32.eqz
    br_if 1
    end
    local.get 1
    local.set 58
    local.get 1
    i32.const 4
    i32.add
    local.set 59
    local.get 58
    local.get 59
    call 0
    local.get 52
    local.set 60
    end
    block
    block
    i32.const 1
    local.set 61
    local.get 60
    local.get 61
    i32.add
    local.set 62
    local.get 1
    i32.const 4
    i32.add
    local.tee 63
    i32.load offset=4
    local.set 64
    local.get 1
    i32.const 8
    i32.add
    local.tee 65
    i32.load offset=8
    local.set 66
    local.get 64
    local.get 66
    i32.gt_s
    local.set 67
    local.get 62
    local.set 70
    local.get 67
    i32.eqz
    br_if 1
    end
    local.get 1
    i32.const 4
    i32.add
    local.set 68
    local.get 1
    i32.const 8
    i32.add
    local.set 69
    local.get 68
    local.get 69
    call 0
    local.get 62
    local.set 70
    end
    block
    block
    i32.const 1
    local.set 71
    local.get 70
    local.get 71
    i32.add
    local.set 72
    local.get 1
    i32.const 8
    i32.add
    local.tee 73
    i32.load offset=8
    local.set 74
    local.get 1
    i32.const 12
    i32.add
    local.tee 75
    i32.load offset=12
    local.set 76
    local.get 74
    local.get 76
    i32.gt_s
    local.set 77
    local.get 72
    local.set 80
    local.get 77
    i32.eqz
    br_if 1
    end
    local.get 1
    i32.const 8
    i32.add
    local.set 78
    local.get 1
    i32.const 12
    i32.add
    local.set 79
    local.get 78
    local.get 79
    call 0
    local.get 72
    local.set 80
    end
    block
    block
    i32.const 1
    local.set 81
    local.get 80
    local.get 81
    i32.add
    local.set 82
    local.get 1
    local.tee 83
    i32.load offset=0
    local.set 84
    local.get 1
    i32.const 4
    i32.add
    local.tee 85
    i32.load offset=4
    local.set 86
    local.get 84
    local.get 86
    i32.gt_s
    local.set 87
    local.get 82
    local.set 90
    local.get 87
    i32.eqz
    br_if 1
    end
    local.get 1
    local.set 88
    local.get 1
    i32.const 4
    i32.add
    local.set 89
    local.get 88
    local.get 89
    call 0
    local.get 82
    local.set 90
    end
    block
    block
    i32.const 1
    local.set 91
    local.get 90
    local.get 91
    i32.add
    local.set 92
    local.get 1
    i32.const 4
    i32.add
    local.tee 93
    i32.load offset=4
    local.set 94
    local.get 1
    i32.const 8
    i32.add
    local.tee 95
    i32.load offset=8
    local.set 96
    local.get 94
    local.get 96
    i32.gt_s
    local.set 97
    local.get 92
    local.set 100
    local.get 97
    i32.eqz
    br_if 1
    end
    local.get 1
    i32.const 4
    i32.add
    local.set 98
    local.get 1
    i32.const 8
    i32.add
    local.set 99
    local.get 98
    local.get 99
    call 0
    local.get 92
    local.set 100
    end
    block
    block
    i32.const 1
    local.set 101
    local.get 100
    local.get 101
    i32.add
    local.set 102
    local.get 1
    local.tee 103
    i32.load offset=0
    local.set 104
    local.get 1
    i32.const 4
    i32.add
    local.tee 105
    i32.load offset=4
    local.set 106
    local.get 104
    local.get 106
    i32.gt_s
    local.set 107
    local.get 102
    local.set 110
    local.get 107
    i32.eqz
    br_if 1
    end
    local.get 1
    local.set 108
    local.get 1
    i32.const 4
    i32.add
    local.set 109
    local.get 108
    local.get 109
    call 0
    local.get 102
    local.set 110
    end
    i32.const 1
    local.set 111
    local.get 110
    local.get 111
    i32.add
    local.set 112
    local.get 1
    i32.const 32
    i32.add
    global.set 0
    local.get 112
    return
    end
  )
  (func $gcd (type 2) (param i32) (param i32) (result i32)
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
    local.get 0
    local.set 2
    local.get 1
    local.set 3
    end
    loop
    block
    block
    local.get 2
    local.get 3
    i32.ne
    local.tee 4
    br_if 1
    end
    local.get 2
    return
    end
    block
    block
    local.get 2
    local.get 3
    i32.gt_s
    local.set 5
    local.get 3
    local.set 7
    local.get 2
    local.set 8
    local.get 5
    i32.eqz
    br_if 1
    end
    local.get 2
    local.get 3
    i32.sub
    local.set 6
    local.get 3
    local.set 7
    local.get 6
    local.set 8
    end
    block
    block
    local.get 7
    local.get 8
    i32.gt_s
    local.set 9
    local.get 8
    local.set 11
    local.get 7
    local.set 12
    local.get 9
    i32.eqz
    br_if 1
    end
    local.get 7
    local.get 8
    i32.sub
    local.set 10
    local.get 8
    local.set 11
    local.get 10
    local.set 12
    end
    local.get 11
    local.set 2
    local.get 12
    local.set 3
    br 0
    end
    unreachable
    end
  )
  (func $lcm (type 2) (param i32) (param i32) (result i32)
    (local i32)
    (local i32)
    (local i32)
    local.get 0
    local.get 1
    call 2
    local.set 2
    local.get 0
    local.get 2
    i32.div_s
    local.tee 3
    local.get 1
    i32.mul
    local.tee 4
    return
    end
  )
  (func $clamp (type 3) (param i32) (param i32) (param i32) (result i32)
    (local i32)
    (local i32)
    block
    block
    local.get 0
    local.get 1
    i32.lt_s
    local.tee 3
    i32.eqz
    br_if 1
    end
    local.get 1
    return
    end
    block
    block
    local.get 0
    local.get 2
    i32.gt_s
    local.tee 4
    i32.eqz
    br_if 1
    end
    local.get 2
    return
    end
    local.get 0
    return
    end
  )
  (func $isqrt (type 1) (param i32) (result i32)
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
    (local i32)
    block
    block
    i32.const 0
    local.set 1
    local.get 0
    local.get 1
    i32.lt_s
    local.tee 2
    i32.eqz
    br_if 1
    end
    i32.const 0
    local.tee 3
    return
    end
    block
    block
    i32.const 0
    local.set 4
    local.get 0
    local.get 4
    i32.eq
    local.tee 5
    i32.eqz
    br_if 1
    end
    i32.const 0
    local.tee 6
    return
    end
    block
    i32.const 1
    local.set 7
    local.get 0
    local.get 7
    i32.add
    local.set 8
    i32.const 2
    local.set 9
    local.get 8
    local.get 9
    i32.div_s
    local.tee 10
    local.set 11
    local.get 0
    local.set 12
    local.get 0
    local.set 14
    end
    loop
    block
    block
    local.get 11
    local.get 12
    i32.lt_s
    local.tee 13
    br_if 1
    end
    local.get 12
    return
    end
    local.get 14
    local.get 11
    i32.div_s
    local.set 15
    local.get 11
    local.get 15
    i32.add
    local.set 16
    i32.const 2
    local.set 17
    local.get 16
    local.get 17
    i32.div_s
    local.set 18
    local.get 11
    local.set 12
    local.get 18
    local.set 11
    br 0
    end
    unreachable
    end
  )
  (func $is_prime (type 1) (param i32) (result i32)
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
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    (local i32)
    block
    block
    i32.const 2
    local.set 1
    local.get 0
    local.get 1
    i32.lt_s
    local.tee 2
    i32.eqz
    br_if 1
    end
    i32.const 0
    local.tee 3
    return
    end
    block
    block
    i32.const 2
    local.set 4
    local.get 0
    local.get 4
    i32.eq
    local.tee 5
    i32.eqz
    br_if 1
    end
    i32.const 1
    local.tee 6
    return
    end
    block
    block
    i32.const 2
    local.set 7
    local.get 0
    local.get 7
    i32.rem_s
    local.set 8
    i32.const 0
    local.set 9
    local.get 8
    local.get 9
    i32.eq
    local.tee 10
    i32.eqz
    br_if 1
    end
    i32.const 0
    local.tee 11
    return
    end
    block
    i32.const 3
    local.set 12
    local.get 0
    call 5
    local.set 13
    local.get 12
    local.set 14
    local.get 13
    local.set 15
    local.get 0
    local.set 17
    end
    loop
    block
    block
    local.get 14
    local.get 15
    i32.le_s
    local.tee 16
    br_if 1
    end
    i32.const 1
    local.tee 24
    return
    end
    block
    block
    local.get 17
    local.get 14
    i32.rem_s
    local.set 18
    i32.const 0
    local.set 19
    local.get 18
    local.get 19
    i32.eq
    local.tee 20
    i32.eqz
    br_if 1
    end
    i32.const 0
    local.tee 21
    return
    end
    i32.const 2
    local.set 22
    local.get 14
    local.get 22
    i32.add
    local.tee 23
    local.set 14
    br 0
    end
    unreachable
    end
  )
  (func $count_primes (type 1) (param i32) (result i32)
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
    i32.const 2
    local.tee 2
    local.set 3
    local.get 0
    local.set 4
    local.get 1
    local.set 9
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
    local.get 9
    return
    end
    block
    block
    local.get 3
    call 6
    local.set 6
    i32.const 1
    local.set 7
    local.get 6
    local.get 7
    i32.eq
    local.set 8
    local.get 3
    local.set 12
    local.get 4
    local.set 15
    local.get 9
    local.set 16
    local.get 8
    i32.eqz
    br_if 1
    end
    i32.const 1
    local.set 10
    local.get 9
    local.get 10
    i32.add
    local.set 11
    local.get 3
    local.set 12
    local.get 4
    local.set 15
    local.get 11
    local.set 16
    end
    i32.const 1
    local.set 13
    local.get 12
    local.get 13
    i32.add
    local.tee 14
    local.set 3
    local.get 15
    local.set 4
    local.get 16
    local.set 9
    br 0
    end
    unreachable
    end
  )
  (func $popcount (type 1) (param i32) (result i32)
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
    local.get 0
    local.set 2
    local.get 1
    local.set 5
    end
    loop
    block
    block
    i32.const 0
    local.set 3
    local.get 2
    local.get 3
    i32.ne
    local.tee 4
    br_if 1
    end
    local.get 5
    return
    end
    i32.const 1
    local.set 6
    local.get 2
    local.get 6
    i32.and
    local.set 7
    local.get 5
    local.get 7
    i32.add
    local.set 8
    i32.const 1
    local.set 9
    local.get 2
    local.get 9
    i32.shr_s
    local.tee 10
    local.set 2
    local.get 8
    local.set 5
    br 0
    end
    unreachable
    end
  )
  (func $reverse_bits (type 1) (param i32) (result i32)
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
    local.get 1
    local.set 6
    local.get 0
    local.set 9
    end
    loop
    block
    block
    i32.const 32
    local.set 4
    local.get 3
    local.get 4
    i32.lt_s
    local.tee 5
    br_if 1
    end
    local.get 6
    return
    end
    i32.const 1
    local.set 7
    local.get 6
    local.get 7
    i32.shl
    local.set 8
    i32.const 1
    local.set 10
    local.get 9
    local.get 10
    i32.and
    local.set 11
    local.get 8
    local.get 11
    i32.or
    local.set 12
    i32.const 1
    local.set 13
    local.get 9
    local.get 13
    i32.shr_s
    local.set 14
    i32.const 1
    local.set 15
    local.get 3
    local.get 15
    i32.add
    local.tee 16
    local.set 3
    local.get 12
    local.set 6
    local.get 14
    local.set 9
    br 0
    end
    unreachable
    end
  )
  (func $is_power_of_two (type 1) (param i32) (result i32)
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
    block
    i32.const 0
    local.set 1
    local.get 0
    local.get 1
    i32.le_s
    local.tee 2
    i32.eqz
    br_if 1
    end
    i32.const 0
    local.tee 3
    return
    end
    block
    block
    i32.const 1
    local.set 4
    local.get 0
    local.get 4
    i32.sub
    local.set 5
    local.get 0
    local.get 5
    i32.and
    local.set 6
    i32.const 0
    local.set 7
    local.get 6
    local.get 7
    i32.eq
    local.tee 8
    i32.eqz
    br_if 1
    end
    i32.const 1
    local.tee 9
    return
    end
    i32.const 0
    local.tee 10
    return
    end
  )
  (func $pow_int (type 2) (param i32) (param i32) (result i32)
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
    i32.const 1
    local.set 2
    local.get 1
    local.set 3
    local.get 2
    local.set 10
    local.get 0
    local.set 11
    end
    loop
    block
    block
    i32.const 0
    local.set 4
    local.get 3
    local.get 4
    i32.gt_s
    local.tee 5
    br_if 1
    end
    local.get 10
    return
    end
    block
    block
    i32.const 1
    local.set 6
    local.get 3
    local.get 6
    i32.and
    local.set 7
    i32.const 1
    local.set 8
    local.get 7
    local.get 8
    i32.eq
    local.set 9
    local.get 11
    local.set 13
    local.get 3
    local.set 15
    local.get 10
    local.set 18
    local.get 9
    i32.eqz
    br_if 1
    end
    local.get 10
    local.get 11
    i32.mul
    local.set 12
    local.get 11
    local.set 13
    local.get 3
    local.set 15
    local.get 12
    local.set 18
    end
    local.get 13
    local.get 13
    i32.mul
    local.set 14
    i32.const 1
    local.set 16
    local.get 15
    local.get 16
    i32.shr_s
    local.tee 17
    local.set 3
    local.get 18
    local.set 10
    local.get 14
    local.set 11
    br 0
    end
    unreachable
    end
  )
  (func $digit_sum (type 1) (param i32) (result i32)
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
    (local i32)
    (local i32)
    block
    block
    i32.const 0
    local.set 1
    i32.const 0
    local.set 2
    local.get 0
    local.get 2
    i32.lt_s
    local.set 3
    local.get 0
    local.set 6
    local.get 1
    local.set 19
    local.get 3
    i32.eqz
    br_if 1
    end
    i32.const 0
    local.tee 4
    local.get 0
    i32.sub
    local.tee 5
    local.set 6
    local.get 1
    local.set 19
    end
    block
    block
    i32.const 0
    local.set 7
    local.get 6
    local.get 7
    i32.eq
    local.tee 8
    i32.eqz
    br_if 1
    end
    i32.const 0
    local.tee 9
    return
    end
    block
    local.get 6
    local.set 10
    local.get 19
    local.set 13
    end
    loop
    block
    block
    i32.const 0
    local.set 11
    local.get 10
    local.get 11
    i32.gt_s
    local.tee 12
    br_if 1
    end
    local.get 13
    return
    end
    i32.const 10
    local.set 14
    local.get 10
    local.get 14
    i32.rem_s
    local.set 15
    local.get 13
    local.get 15
    i32.add
    local.set 16
    i32.const 10
    local.set 17
    local.get 10
    local.get 17
    i32.div_s
    local.tee 18
    local.set 10
    local.get 16
    local.set 13
    br 0
    end
    unreachable
    end
  )
  (func $collatz_steps (type 1) (param i32) (result i32)
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
    i32.const 0
    local.tee 3
    return
    end
    block
    i32.const 0
    local.set 4
    local.get 0
    local.set 5
    local.get 4
    local.set 24
    end
    loop
    block
    block
    i32.const 1
    local.set 6
    local.get 5
    local.get 6
    i32.gt_s
    local.tee 7
    br_if 1
    end
    local.get 24
    return
    end
    block
    block
    i32.const 1
    local.set 8
    local.get 5
    local.get 8
    i32.and
    local.set 9
    i32.const 0
    local.set 10
    local.get 9
    local.get 10
    i32.eq
    local.set 11
    local.get 9
    local.set 14
    local.get 5
    local.set 18
    local.get 24
    local.set 23
    local.get 11
    i32.eqz
    br_if 1
    end
    i32.const 2
    local.set 12
    local.get 5
    local.get 12
    i32.div_s
    local.set 13
    local.get 9
    local.set 14
    local.get 13
    local.set 18
    local.get 24
    local.set 23
    end
    block
    block
    i32.const 1
    local.set 15
    local.get 14
    local.get 15
    i32.eq
    local.set 16
    local.get 23
    local.set 22
    local.get 18
    local.set 29
    local.get 16
    i32.eqz
    br_if 1
    end
    i32.const 3
    local.tee 17
    local.get 18
    i32.mul
    local.set 19
    i32.const 1
    local.set 20
    local.get 19
    local.get 20
    i32.add
    local.set 21
    local.get 23
    local.set 22
    local.get 21
    local.set 29
    end
    block
    block
    i32.const 1
    local.set 25
    local.get 22
    local.get 25
    i32.add
    local.set 26
    i32.const 10000
    local.set 27
    local.get 26
    local.get 27
    i32.gt_s
    local.tee 28
    i32.eqz
    br_if 1
    end
    local.get 26
    return
    end
    local.get 29
    local.set 5
    local.get 26
    local.set 24
    br 0
    end
    unreachable
    end
  )
  (export "swap" (func 0))
  (export "bubble_sort_count" (func 1))
  (export "gcd" (func 2))
  (export "lcm" (func 3))
  (export "clamp" (func 4))
  (export "isqrt" (func 5))
  (export "is_prime" (func 6))
  (export "count_primes" (func 7))
  (export "popcount" (func 8))
  (export "reverse_bits" (func 9))
  (export "is_power_of_two" (func 10))
  (export "pow_int" (func 11))
  (export "digit_sum" (func 12))
  (export "collatz_steps" (func 13))
  (export "memory" (memory 0))
)
