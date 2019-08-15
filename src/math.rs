use std::fmt::{Debug, Formatter};
use std::ops::{Mul, Add, Neg};

#[derive(Copy, Clone, PartialEq)]
pub struct Vec3(pub vecmath::Vector3<f32>);

impl Debug for Vec3 {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "[{:5.2}, {:5.2}, {:5.2}]", self.0[0], self.0[1], self.0[2])
    }
}

impl Vec3 {
    pub fn xyz1(&self) -> Vec4 {
        Vec4([self.0[0], self.0[1], self.0[2], 1.0])
    }

    pub fn xyz0(&self) -> Vec4 {
        Vec4([self.0[0], self.0[1], self.0[2], 0.0])
    }

    pub fn normalize(&self) -> Vec3 {
        Vec3(vecmath::vec3_normalized(self.0))
    }

    pub fn cross(&self, rhs: &Vec3) -> Vec3 {
        Vec3(vecmath::vec3_cross(self.0, rhs.0))
    }

    pub fn dot(&self, rhs: &Vec3) -> f32 {
        vecmath::vec3_dot(self.0, rhs.0)
    }

    pub fn len(&self) -> f32 {
        vecmath::vec3_len(self.0)
    }
}

impl Mul<Vec3> for f32 {
    type Output = Vec3; 

    fn mul(self, rhs: Vec3) -> Vec3 {
        Vec3(vecmath::vec3_mul([self, self, self], rhs.0))
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3; 

    fn mul(self, rhs: f32) -> Vec3 {
        Vec3(vecmath::vec3_mul(self.0, [rhs, rhs, rhs]))
    }
}

impl Add<Vec3> for Vec3 {
    type Output = Vec3; 

    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3(vecmath::vec3_add(self.0, rhs.0))
    }
}

impl Neg for Vec3 {
    type Output = Vec3; 

    fn neg(self) -> Vec3 {
        Vec3(vecmath::vec3_neg(self.0))
    }
}

#[derive(Copy, Clone, PartialEq)]
pub struct Vec4(pub vecmath::Vector4<f32>);

impl Vec4 {
    pub fn xyz(&self) -> Vec3 {
        Vec3([self.0[0], self.0[1], self.0[2]])
    }
}

impl Debug for Vec4 {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "[{:5.2}, {:5.2}, {:5.2}, {:5.2}]", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

#[derive(Copy, Clone, PartialEq)]
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
        Vec4(vecmath::col_mat4_transform(self.0, rhs.0))
    }
}

impl Mat4 {
    pub fn rotation_around_vector(v: &Vec3, angle: f32 /* in Rad */) -> Mat4 {
        let (x, y, z) = (v.0[0], v.0[1], v.0[2]);
        let a = 1.0 - angle.cos();
        Mat4([
            [x*x*a +   angle.cos(), x*y*a - z*angle.sin(), x*z*a + y*angle.sin(), 0.0],
            [y*x*a + z*angle.sin(), y*y*a +   angle.cos(), y*z*a - x*angle.sin(), 0.0],
            [z*x*a - y*angle.sin(), z*y*a + x*angle.sin(), z*z*a +   angle.cos(), 0.0],
            [0.0,                   0.0,                   0.0,                   1.0],
        ])
    }

    pub fn inv(&self) -> Mat4 {
        Mat4(vecmath::mat4_inv(self.0))
    }

    pub fn transpose(&self) -> Mat4 {
        Mat4(vecmath::mat4_transposed(self.0))
    }
}
