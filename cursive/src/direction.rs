//! Direction-related structures.
//!
//! This module defines two main concepts: [`Orientation`] and [`Direction`].
//!
//! [`Orientation`]: direction::Orientation
//! [`Direction`]: direction::Direction
//!
//! ### Orientation
//!
//! `Orientation` is a simple enum that can take two values:
//! `Horizontal` or `Vertical`.
//!
//! ### Direction
//!
//! `Direction` is a bit more complex, and can be of two kinds:
//!
//! * Absolute direction: left, up, right, or down
//! * Relative direction: front or back. Its actual direction depends on the
//!   orientation.
//!
//!   Usually, "front" refers to the "forward" direction, or the "next"
//!   element. For example, for a vertical `LinearLayout`, "front" would refer
//!   to the "down" direction.
//!
//!   This is mostly relevant when referring to change of focus. Hitting the
//!   `Tab` key would usually cycle focus in the "front" direction, while
//!   using the arrow keys would use absolute directions instead.

use crate::vec::Vec2;
use crate::XY;

/// Describes a vertical or horizontal orientation for a view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Orientation {
    /// Horizontal orientation
    Horizontal,
    /// Vertical orientation
    Vertical,
}

impl Orientation {
    /// Returns a `XY(Horizontal, Vertical)`.
    pub fn pair() -> XY<Orientation> {
        XY::new(Orientation::Horizontal, Orientation::Vertical)
    }

    /// Returns the component of `v` corresponding to this orientation.
    ///
    /// (`Horizontal` will return the x value,
    /// and `Vertical` will return the y value.)
    pub fn get<T: Clone>(self, v: &XY<T>) -> T {
        v.get(self).clone()
    }

    /// Returns the other orientation.
    pub fn swap(self) -> Self {
        match self {
            Orientation::Horizontal => Orientation::Vertical,
            Orientation::Vertical => Orientation::Horizontal,
        }
    }

    /// Returns a mutable reference to the component of the given vector
    /// corresponding to this orientation.
    ///
    /// # Examples
    /// ```rust
    /// # use cursive::XY;
    /// # use cursive::direction::Orientation;
    /// let o = Orientation::Horizontal;
    /// let mut xy = XY::new(1, 2);
    /// *o.get_ref(&mut xy) = 42;
    ///
    /// assert_eq!(xy, XY::new(42, 2));
    /// ```
    pub fn get_ref<T>(self, v: &mut XY<T>) -> &mut T {
        match self {
            Orientation::Horizontal => &mut v.x,
            Orientation::Vertical => &mut v.y,
        }
    }

    /// Takes an iterator on sizes, and stack them in the current orientation,
    /// returning the size of the required bounding box.
    ///
    /// * For an horizontal view, returns `(Sum(x), Max(y))`.
    /// * For a vertical view, returns `(Max(x), Sum(y))`.
    pub fn stack<'a, T: Iterator<Item = &'a Vec2>>(self, iter: T) -> Vec2 {
        match self {
            Orientation::Horizontal => {
                iter.fold(Vec2::zero(), |a, b| a.stack_horizontal(b))
            }
            Orientation::Vertical => {
                iter.fold(Vec2::zero(), |a, b| a.stack_vertical(b))
            }
        }
    }

    /// Creates a new `Vec2` with `main_axis` in `self`'s axis, and
    /// `second_axis` for the other axis.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cursive::direction::Orientation;
    /// # use cursive::vec::Vec2;
    /// let o = Orientation::Horizontal;
    /// let vec = o.make_vec(1, 2);
    ///
    /// assert_eq!(vec, Vec2::new(1, 2));
    ///
    /// let o = Orientation::Vertical;
    /// let vec = o.make_vec(1, 2);
    ///
    /// assert_eq!(vec, Vec2::new(2, 1));
    /// ```
    pub fn make_vec(self, main_axis: usize, second_axis: usize) -> Vec2 {
        let mut result = Vec2::zero();
        *self.get_ref(&mut result) = main_axis;
        *self.swap().get_ref(&mut result) = second_axis;
        result
    }
}

/// Represents a direction, either absolute or orientation-dependent.
///
/// * Absolute directions are Up, Down, Left, and Right.
/// * Relative directions are Front and Back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    /// An absolute direction.
    Abs(Absolute),
    /// A direction relative to the current orientation.
    Rel(Relative),
}

impl Direction {
    /// Returns the relative direction for the given orientation.
    ///
    /// Some combination have no corresponding relative position. For example,
    /// `Direction::Abs(Up)` means nothing for `Orientation::Horizontal`.
    pub fn relative(self, orientation: Orientation) -> Option<Relative> {
        match self {
            Direction::Abs(abs) => abs.relative(orientation),
            Direction::Rel(rel) => Some(rel),
        }
    }

    /// Returns the absolute direction in the given `orientation`.
    pub fn absolute(self, orientation: Orientation) -> Absolute {
        match self {
            Direction::Abs(abs) => abs,
            Direction::Rel(rel) => rel.absolute(orientation),
        }
    }

    /// Shortcut to create `Direction::Rel(Relative::Back)`
    pub fn back() -> Self {
        Direction::Rel(Relative::Back)
    }

    /// Shortcut to create `Direction::Rel(Relative::Front)`
    pub fn front() -> Self {
        Direction::Rel(Relative::Front)
    }

    /// Shortcut to create `Direction::Abs(Absolute::Left)`
    pub fn left() -> Self {
        Direction::Abs(Absolute::Left)
    }

    /// Shortcut to create `Direction::Abs(Absolute::Right)`
    pub fn right() -> Self {
        Direction::Abs(Absolute::Right)
    }

    /// Shortcut to create `Direction::Abs(Absolute::Up)`
    pub fn up() -> Self {
        Direction::Abs(Absolute::Up)
    }

    /// Shortcut to create `Direction::Abs(Absolute::Down)`
    pub fn down() -> Self {
        Direction::Abs(Absolute::Down)
    }

    /// Shortcut to create `Direction::Abs(Absolute::None)`
    pub fn none() -> Self {
        Direction::Abs(Absolute::None)
    }
}

/// Direction relative to an orientation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Relative {
    // TODO: handle right-to-left? (Arabic, ...)
    /// Front relative direction.
    ///
    /// * Horizontally, this means `Left`
    /// * Vertically, this means `Up`
    Front,

    /// Back relative direction.
    ///
    /// * Horizontally, this means `Right`
    /// * Vertically, this means `Down`.
    Back,
}

impl Relative {
    /// Returns the absolute direction in the given `orientation`.
    pub fn absolute(self, orientation: Orientation) -> Absolute {
        match (orientation, self) {
            (Orientation::Horizontal, Relative::Front) => Absolute::Left,
            (Orientation::Horizontal, Relative::Back) => Absolute::Right,
            (Orientation::Vertical, Relative::Front) => Absolute::Up,
            (Orientation::Vertical, Relative::Back) => Absolute::Down,
        }
    }
}

/// Absolute direction (up, down, left, right).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Absolute {
    /// Left
    Left,
    /// Up
    Up,
    /// Right
    Right,
    /// Down
    Down,

    /// No real direction.
    ///
    /// Used when the "direction" is accross layers for instance.
    None,
}

impl Absolute {
    /// Returns the relative direction for the given orientation.
    ///
    /// Returns `None` when the direction does not apply to the given
    /// orientation (ex: `Left` and `Vertical`).
    pub fn relative(self, orientation: Orientation) -> Option<Relative> {
        match (orientation, self) {
            (Orientation::Horizontal, Absolute::Left)
            | (Orientation::Vertical, Absolute::Up) => Some(Relative::Front),
            (Orientation::Horizontal, Absolute::Right)
            | (Orientation::Vertical, Absolute::Down) => Some(Relative::Back),
            _ => None,
        }
    }
}
