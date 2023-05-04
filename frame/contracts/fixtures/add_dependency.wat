;; Just delegate call into the passed code hash and assert success.
(module
	(import "seal0" "add_dependency" (func $add_dependency (param i32) (result i32)))
	(import "seal0" "remove_dependency" (func $remove_dependency (param i32) (result i32)))
	(import "seal0" "seal_input" (func $seal_input (param i32 i32)))
	(import "seal0" "seal_delegate_call" (func $seal_delegate_call (param i32 i32 i32 i32 i32 i32) (result i32)))
	(import "env" "memory" (memory 1 1))

	(func $assert (param i32)
		(block $ok
			(br_if $ok
				(get_local 0)
			)
			(unreachable)
		)
	)

	(func $load_input
	    ;; Store available input size at offset 0.
        (i32.store (i32.const 0) (i32.const 512))

		;; read input data
		(call $seal_input (i32.const 4) (i32.const 0))

		;; Input data layout.
		;; [0..4) - size of the call
		;; [4..38) - code hash of the callee

		;; assert input size == 32
		(call $assert
			(i32.eq
				(i32.load (i32.const 0))
				(i32.const 32)
			)
		)
	)

	(func (export "deploy")
		(call $load_input)

		;; call add dependency
		(call $assert (i32.eqz
			(call $add_dependency
				(i32.const 4) ;; Pointer to hash
			)
		))
	)

	(func (export "call")
		(call $load_input)

		;; Delegate call into passed code hash
		(call $assert
			(i32.eq
				(call $seal_delegate_call
					(i32.const 0)	;; Set no call flags
					(i32.const 4)	;; Pointer to "callee" code_hash.
					(i32.const 0)	;; Input is ignored
					(i32.const 0)	;; Length of the input
					(i32.const 4294967295)	;; u32 max sentinel value: do not copy output
					(i32.const 0)	;; Length is ignored in this case
				)
				(i32.const 0)
			)
		)

		;; call add remove_dependency
		(call $assert (i32.eqz
			(call $remove_dependency
				(i32.const 4) ;; Pointer to hash
			)
		))
	)

)
