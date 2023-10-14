.altmacro
.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm

.align 2
__alltraps:
    csrrw sp, sscratch, sp # sp -> sscratch -> sp

    # allocate a frame for trap context on kernel stack
    # trap context is 34-dword:
    # - 32-dword: general registers
    # - 1-dword: sstatus
    # - 1-dword: sepc
    addi sp, sp, -34*8

    # save general registers
    sd x1, 1*8(sp)
    sd x3, 3*8(sp)
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr

    # save sstatus & sepc
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    # read user stack from sscratch and save it on the kernel stack
    csrr t2, sscratch
    sd t2, 2*8(sp)

    # set input argument.
    # trap_handler(ctx: &mut TrapContext)
    mv a0, sp
    call trap_handler

.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm

__restore:
    mv sp, a0

    # restore csr
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    ld t2, 2*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    csrw sscratch, t2

    # restore general registers
    ld x1, 1*8(sp)
    ld x3, 1*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr

    # release kernel stack frame
    addi sp, sp, 34*8
    csrrw sp, sscratch, sp

    # return to U/S
    sret
