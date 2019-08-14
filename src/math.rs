use std::fmt::{Debug, Formatter};
use std::ops::Mul;

pub type Vec3 = vecmath::Vector3<f32>;

pub type Vec4 = vecmath::Vector4<f32>;

#[derive(Copy, Clone)]
pub struct Mat4(pub vecmath::Matrix4<f32>); // column major

impl Debug for Mat4 {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f, "{:5.2} {:5.2} {:5.2} {:5.2}\n{:5.2} {:5.2} {:5.2} {:5.2}\n{:5.2} {:5.2} {:5.2} {:5.2}\n{:5.2} {:5.2} {:5.2} {:5.2}",
            self.0[0][0], self.0[1][0], self.0[2][0], self.0[3][0], 
            self.0[0][1], self.0[1][1], self.0[2][1], self.0[3][1], 
            self.0[0][2], self.0[1][2], self.0[2][2], self.0[3][2],
            self.0[0][3], self.0[1][3], self.0[2][3], self.0[3][3],
        )
    }
}

impl Mul<Mat4> for Mat4 {
    type Output = Mat4; 

    fn mul(self, rhs: Mat4) -> Mat4 {
        Mat4(vecmath::col_mat4_mul(self.0, rhs.0))
    }
}

impl Mul<Vec4> for Mat4 {
    type Output = Vec4; 

    fn mul(self, rhs: Vec4) -> Vec4 {
        vecmath::col_mat4_transform(self.0, rhs)
    }
}
