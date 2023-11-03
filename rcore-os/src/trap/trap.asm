.altmacro
.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm

    .section .text.trampoline
    .globl __restore
    .globl __alltraps
    .align 2
__alltraps:
    csrrw sp, sscratch, sp # sp -> sscratch -> sp

    # Trap context is stored on the second highest page and the address is stored in sscratch
    # trap context is 34-dword:
    # - 32-dword: general registers
    # - 1-dword: sstatus
    # - 1-dword: sepc
    # - 1-dword: kernel satp
    # - 1-dword: kernel sp
    # - 1-dword: trap handler address

    # save general registers
    # skip sp(x2) and tp(x4)
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

    # load kernel satp, kernel sp and trap handler address
    ld t0, 34*8(sp)
    ld t1, 36*8(sp)
    ld sp, 35*8(sp)

    # set kernel address space
    csrw satp, t0
    sfence.vma

    jr t1

.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm

__restore:
    # a0: *TrapContext in user space a1: user space token
    csrw satp, a1
    sfence.vma
    csrw ssratch, a0
    mv sp, a0

    # restore csr
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    csrw sstatus, t0
    csrw sepc, t1

    # restore general registers
    ld x1, 1*8(sp)
    ld x3, 1*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr

    ld sp, 2*8(sp)

    # return to U/S
    sret
