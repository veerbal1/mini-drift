use crate::error::{ErrorCode, MiniDriftResult};

pub trait SafeMath: Sized {
    fn safe_add(self, rhs: Self) -> MiniDriftResult<Self>;
    fn safe_sub(self, rhs: Self) -> MiniDriftResult<Self>;
    fn safe_mul(self, rhs: Self) -> MiniDriftResult<Self>;
    fn safe_div(self, rhs: Self) -> MiniDriftResult<Self>;
}

impl SafeMath for u64 {
    fn safe_add(self, rhs: Self) -> MiniDriftResult<Self> {
        self.checked_add(rhs).ok_or(ErrorCode::MathError)
    }

    fn safe_sub(self, rhs: Self) -> MiniDriftResult<Self> {
        self.checked_sub(rhs).ok_or(ErrorCode::MathError)
    }

    fn safe_mul(self, rhs: Self) -> MiniDriftResult<Self> {
        self.checked_mul(rhs).ok_or(ErrorCode::MathError)
    }

    fn safe_div(self, rhs: Self) -> MiniDriftResult<Self> {
        self.checked_div(rhs).ok_or(ErrorCode::MathError)
    }
}

impl SafeMath for u128 {
    fn safe_add(self, rhs: Self) -> MiniDriftResult<Self> {
        self.checked_add(rhs).ok_or(ErrorCode::MathError)
    }

    fn safe_sub(self, rhs: Self) -> MiniDriftResult<Self> {
        self.checked_sub(rhs).ok_or(ErrorCode::MathError)
    }

    fn safe_mul(self, rhs: Self) -> MiniDriftResult<Self> {
        self.checked_mul(rhs).ok_or(ErrorCode::MathError)
    }

    fn safe_div(self, rhs: Self) -> MiniDriftResult<Self> {
        self.checked_div(rhs).ok_or(ErrorCode::MathError)
    }
}
