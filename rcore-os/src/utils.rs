pub unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
        core::slice::from_raw_parts(
            (p as *const T) as *const u8,
            core::mem::size_of::<T>(),
        )
}

pub fn copy_to_dsts(mut src: &[u8], dsts: &mut [&mut [u8]]) -> Result<(), ()> {
    let len: usize = dsts.iter().map(|b| b.len()).sum();
    if src.len() != len {
        return Err(());
    }

    for dst in dsts {
        dst.copy_from_slice(src);
        let len = dst.len();
        src = &src[len..];
    }

    Ok(())
}
