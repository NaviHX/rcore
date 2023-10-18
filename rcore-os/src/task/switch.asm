.altmacro
.macro SAVE_SN n
    sd s\n, (\n+2)*8(a0)
.endm
.macro LOAD_SN n
    ld s\n, (\n+2)*8(a1)
.endm

    .section .text
    .globl __switch
# Check function signatrue in switch.rs
__switch:
    # Save sp
    sd sp, 8(a0)

    # Save ra
    sd ra, 0(a0)

    # Save sn
    .set n, 0
    .rept 12
        SAVE_SN %n
        .set n, n+1
    .endr

    # Load sp, ra, sn from the next task context
    ld sp, 8(a1)
    ld ra, 0(a1)
    .set n, 0
    .rept 12
        LOAD_SN %n
        .set n, n+1
    .endr

    # Because sp has been changed to the new context,
    # this ret will go back to the next task's control flow
    ret
