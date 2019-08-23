use std::fmt::{Debug, Formatter};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, Neg, Sub};

#[derive(Copy, Clone, PartialEq)]
pub struct Vec3(pub vecmath::Vector3<f64>);

impl Debug for Vec3 {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "[{:5.2}, {:5.2}, {:5.2}]", self.0[0], self.0[1], self.0[2])
    }
}

impl Vec3 {
    #[inline(always)]
    pub fn xyz1(self) -> Vec4 {
        Vec4([self.0[0], self.0[1], self.0[2], 1.0])
    }

    #[inline(always)]
    pub fn xyz0(self) -> Vec4 {
        Vec4([self.0[0], self.0[1], self.0[2], 0.0])
    }

    #[inline(always)]
    pub fn normalize(self) -> Vec3 {
        Vec3(vecmath::vec3_normalized(self.0))
    }

    #[inline(always)]
    pub fn cross(self, rhs: Vec3) -> Vec3 {
        Vec3(vecmath::vec3_cross(self.0, rhs.0))
    }

    #[inline(always)]
    pub fn dot(self, rhs: Vec3) -> f64 {
        vecmath::vec3_dot(self.0, rhs.0)
    }

    #[inline(always)]
    pub fn len(self) -> f64 {
        vecmath::vec3_len(self.0)
    }

    #[inline(always)]
    pub fn sqlen(self) -> f64 {
        vecmath::vec3_square_len(self.0)
    }

    #[inline(always)]
    pub fn x(self) -> f64 {
        self.0[0]
    }

    #[inline(always)]
    pub fn y(self) -> f64 {
        self.0[1]
    }

    #[inline(always)]
    pub fn z(self) -> f64 {
        self.0[2]
    }

    #[inline(always)]
    pub fn min(self, other: Vec3) -> Vec3 {
        Vec3([self.0[0].min(other.0[0]), self.0[1].min(other.0[1]), self.0[2].min(other.0[2])])
    }

    #[inline(always)]
    pub fn max(self, other: Vec3) -> Vec3 {
        Vec3([self.0[0].max(other.0[0]), self.0[1].max(other.0[1]), self.0[2].max(other.0[2])])
    }

    #[inline(always)]
    pub fn normalize_len(self) -> (Vec3, f64) {
        let len = vecmath::vec3_len(self.0);
        (Vec3([self.0[0] / len, self.0[1] / len, self.0[2] / len]), len)
    }
}

impl Mul<Vec3> for f64 {
    type Output = Vec3;

    #[inline(always)]
    fn mul(self, rhs: Vec3) -> Vec3 {
        Vec3(vecmath::vec3_mul([self, self, self], rhs.0))
    }
}

impl Mul<f64> for Vec3 {
    type Output = Vec3;

    #[inline(always)]
    fn mul(self, rhs: f64) -> Vec3 {
        Vec3(vecmath::vec3_mul(self.0, [rhs, rhs, rhs]))
    }
}

impl Mul<Vec3> for Vec3 {
    type Output = Vec3;

    #[inline(always)]
    fn mul(self, rhs: Vec3) -> Vec3 {
        Vec3(vecmath::vec3_mul(self.0, rhs.0))
    }
}

impl Add<Vec3> for Vec3 {
    type Output = Vec3;

    #[inline(always)]
    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3(vecmath::vec3_add(self.0, rhs.0))
    }
}

impl Neg for Vec3 {
    type Output = Vec3;

    #[inline(always)]
    fn neg(self) -> Vec3 {
        Vec3(vecmath::vec3_neg(self.0))
    }
}

impl Sub<Vec3> for Vec3 {
    type Output = Vec3;

    #[inline(always)]
    fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3(vecmath::vec3_sub(self.0, rhs.0))
    }
}

impl AddAssign<Vec3> for Vec3 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Vec3) {
        self.0 = vecmath::vec3_add(self.0, rhs.0);
    }
}

impl DivAssign<f64> for Vec3 {
    #[inline(always)]
    fn div_assign(&mut self, rhs: f64) {
        self.0[0] /= rhs;
        self.0[1] /= rhs;
        self.0[2] /= rhs;
    }
}

impl Div<f64> for Vec3 {
    type Output = Vec3;
    #[inline(always)]
    fn div(self, rhs: f64) -> Vec3 {
        Vec3([self.0[0] / rhs, self.0[1] / rhs, self.0[2] / rhs])
    }
}

#[derive(Copy, Clone, PartialEq)]
pub struct Vec4(pub vecmath::Vector4<f64>);

impl Vec4 {
    #[inline(always)]
    pub fn xyz(self) -> Vec3 {
        Vec3([self.0[0], self.0[1], self.0[2]])
    }
}

impl Debug for Vec4 {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "[{:5.2}, {:5.2}, {:5.2}, {:5.2}]", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

#[derive(Copy, Clone, PartialEq)]
pub struct Mat4(pub vecmath::Matrix4<f64>); // column major

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

    #[inline(always)]
    fn mul(self, rhs: Mat4) -> Mat4 {
        Mat4(vecmath::col_mat4_mul(self.0, rhs.0))
    }
}

impl Mul<Vec4> for Mat4 {
    type Output = Vec4;

    #[inline(always)]
    fn mul(self, rhs: Vec4) -> Vec4 {
        Vec4(vecmath::col_mat4_transform(self.0, rhs.0))
    }
}

impl Mat4 {
    #[inline(always)]
    pub fn rotation_around_vector(axis: Vec3, angle: f64 /* in rad */) -> Mat4 {
        let (x, y, z) = (axis.0[0], axis.0[1], axis.0[2]);
        let a = 1.0 - angle.cos();
        Mat4([
            [
                x * x * a + angle.cos(),
                x * y * a - z * angle.sin(),
                x * z * a + y * angle.sin(),
                0.0,
            ],
            [
                y * x * a + z * angle.sin(),
                y * y * a + angle.cos(),
                y * z * a - x * angle.sin(),
                0.0,
            ],
            [
                z * x * a - y * angle.sin(),
                z * y * a + x * angle.sin(),
                z * z * a + angle.cos(),
                0.0,
            ],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    #[inline(always)]
    pub fn inv(self) -> Mat4 {
        Mat4(vecmath::mat4_inv(self.0))
    }

    #[inline(always)]
    pub fn transpose(self) -> Mat4 {
        Mat4(vecmath::mat4_transposed(self.0))
    }
}

pub const EPS: f64 = 2e-7;

pub trait AlmostEq {
    fn almost_eq(self, rhs: Self) -> bool;
    fn almost_zero(self) -> bool;
}

impl AlmostEq for f64 {
    #[inline(always)]
    fn almost_eq(self, rhs: f64) -> bool {
        (self - rhs).abs() < EPS
    }

    #[inline(always)]
    fn almost_zero(self) -> bool {
        self.abs() < EPS
    }
}

pub trait HasAABB {
    // The first Vec3 has to be the minimum coordinate of the AABB, the second the maximum.
    fn calculate_aabb(&self) -> (Vec3, Vec3);
}
