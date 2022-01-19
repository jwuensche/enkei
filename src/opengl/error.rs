use log::error;


pub unsafe fn check_error(pos: &str) {
    let err = gl::GetError();
    if err != 0 {
        error!("OpenGL has encountered an error ({}) ({})", err, pos);
        error!("GL_INVALID_VALUE: {}", gl::INVALID_VALUE);
        error!("GL_INVALID_ENUM: {}", gl::INVALID_ENUM);
        error!("GL_INVALID_OPERATION: {}", gl::INVALID_OPERATION);
        std::process::exit(1);
    }
}
