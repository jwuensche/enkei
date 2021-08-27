pub unsafe fn check_error(pos: &str) {
    let err = gl::GetError();
    if err != 0 {
        eprintln!("OpenGL has encountered an error ({}) ({})", err, pos);
        eprintln!("GL_INVALID_VALUE: {}", gl::INVALID_VALUE);
        eprintln!("GL_INVALID_ENUM: {}", gl::INVALID_ENUM);
        eprintln!("GL_INVALID_OPERATION: {}", gl::INVALID_OPERATION);
        std::process::exit(1);
    }
}
