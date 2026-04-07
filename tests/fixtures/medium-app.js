// Medium-size application module (~200 lines) for benchmarking
// Simulates a typical Express.js route handler with auth, validation, DB ops

const express = require('express');
const router = express.Router();

class ValidationError extends Error {
  constructor(field, message) {
    super(message);
    this.field = field;
    this.status = 400;
  }
}

function validateEmail(email) {
  if (!email || typeof email !== 'string') return false;
  const parts = email.split('@');
  if (parts.length !== 2) return false;
  const [local, domain] = parts;
  return local.length > 0 && domain.includes('.') && domain.length > 2;
}

function validatePassword(password) {
  if (!password || typeof password !== 'string') return false;
  if (password.length < 8) return false;
  const hasUpper = /[A-Z]/.test(password);
  const hasLower = /[a-z]/.test(password);
  const hasNumber = /[0-9]/.test(password);
  return hasUpper && hasLower && hasNumber;
}

function sanitizeInput(input) {
  if (typeof input !== 'string') return input;
  return input
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#x27;');
}

const authenticate = async (req, res, next) => {
  const token = req.headers.authorization?.split(' ')[1];
  if (!token) {
    return res.status(401).json({ error: 'Authentication required' });
  }
  try {
    const decoded = jwt.verify(token, process.env.JWT_SECRET ?? 'default-secret');
    req.user = decoded;
    next();
  } catch (err) {
    if (err.name === 'TokenExpiredError') {
      return res.status(401).json({ error: 'Token expired', code: 'TOKEN_EXPIRED' });
    }
    return res.status(403).json({ error: 'Invalid token' });
  }
};

const authorize = (...roles) => {
  return (req, res, next) => {
    if (!req.user) {
      return res.status(401).json({ error: 'Not authenticated' });
    }
    if (roles.length > 0 && !roles.includes(req.user.role)) {
      return res.status(403).json({ error: 'Insufficient permissions' });
    }
    next();
  };
};

router.post('/users', authenticate, authorize('admin'), async (req, res) => {
  try {
    const { email, password, name, role } = req.body;

    if (!validateEmail(email)) {
      throw new ValidationError('email', 'Invalid email address');
    }
    if (!validatePassword(password)) {
      throw new ValidationError('password', 'Password must be 8+ chars with upper, lower, and number');
    }
    if (!name || name.trim().length === 0) {
      throw new ValidationError('name', 'Name is required');
    }

    const sanitizedName = sanitizeInput(name.trim());
    const existingUser = await db.users.findOne({ email: email.toLowerCase() });

    if (existingUser) {
      return res.status(409).json({ error: 'Email already registered' });
    }

    const hashedPassword = await bcrypt.hash(password, 12);
    const user = await db.users.create({
      email: email.toLowerCase(),
      password: hashedPassword,
      name: sanitizedName,
      role: role ?? 'user',
      createdAt: new Date(),
      updatedAt: new Date(),
    });

    const { password: _, ...userResponse } = user.toJSON();
    res.status(201).json({ data: userResponse });
  } catch (err) {
    if (err instanceof ValidationError) {
      return res.status(err.status).json({
        error: err.message,
        field: err.field,
      });
    }
    console.error('Failed to create user:', err);
    res.status(500).json({ error: 'Internal server error' });
  }
});

router.get('/users', authenticate, async (req, res) => {
  try {
    const { page = 1, limit = 20, sort = 'createdAt', order = 'desc', search } = req.query;
    const pageNum = Math.max(1, parseInt(page, 10) || 1);
    const limitNum = Math.min(100, Math.max(1, parseInt(limit, 10) || 20));
    const skip = (pageNum - 1) * limitNum;

    const query = {};
    if (search) {
      const sanitized = sanitizeInput(search);
      query.$or = [
        { name: { $regex: sanitized, $options: 'i' } },
        { email: { $regex: sanitized, $options: 'i' } },
      ];
    }

    const sortObj = {};
    const validSortFields = ['name', 'email', 'createdAt', 'role'];
    if (validSortFields.includes(sort)) {
      sortObj[sort] = order === 'asc' ? 1 : -1;
    } else {
      sortObj.createdAt = -1;
    }

    const [users, total] = await Promise.all([
      db.users.find(query).sort(sortObj).skip(skip).limit(limitNum).select('-password'),
      db.users.countDocuments(query),
    ]);

    res.json({
      data: users,
      pagination: {
        page: pageNum,
        limit: limitNum,
        total,
        pages: Math.ceil(total / limitNum),
        hasNext: pageNum * limitNum < total,
        hasPrev: pageNum > 1,
      },
    });
  } catch (err) {
    console.error('Failed to list users:', err);
    res.status(500).json({ error: 'Internal server error' });
  }
});

router.put('/users/:id', authenticate, async (req, res) => {
  try {
    const { id } = req.params;
    const { name, email, role } = req.body;
    const updates = { updatedAt: new Date() };

    if (name !== undefined) {
      if (name.trim().length === 0) {
        throw new ValidationError('name', 'Name cannot be empty');
      }
      updates.name = sanitizeInput(name.trim());
    }

    if (email !== undefined) {
      if (!validateEmail(email)) {
        throw new ValidationError('email', 'Invalid email address');
      }
      const existing = await db.users.findOne({ email: email.toLowerCase(), _id: { $ne: id } });
      if (existing) {
        return res.status(409).json({ error: 'Email already in use' });
      }
      updates.email = email.toLowerCase();
    }

    if (role !== undefined) {
      if (req.user.role !== 'admin') {
        return res.status(403).json({ error: 'Only admins can change roles' });
      }
      const validRoles = ['user', 'admin', 'moderator'];
      if (!validRoles.includes(role)) {
        throw new ValidationError('role', `Invalid role. Must be one of: ${validRoles.join(', ')}`);
      }
      updates.role = role;
    }

    const user = await db.users.findByIdAndUpdate(id, { $set: updates }, { new: true }).select('-password');
    if (!user) {
      return res.status(404).json({ error: 'User not found' });
    }

    res.json({ data: user });
  } catch (err) {
    if (err instanceof ValidationError) {
      return res.status(err.status).json({ error: err.message, field: err.field });
    }
    console.error('Failed to update user:', err);
    res.status(500).json({ error: 'Internal server error' });
  }
});

router.delete('/users/:id', authenticate, authorize('admin'), async (req, res) => {
  try {
    const { id } = req.params;
    if (id === req.user.id) {
      return res.status(400).json({ error: 'Cannot delete your own account' });
    }
    const user = await db.users.findByIdAndDelete(id);
    if (!user) {
      return res.status(404).json({ error: 'User not found' });
    }
    res.status(204).send();
  } catch (err) {
    console.error('Failed to delete user:', err);
    res.status(500).json({ error: 'Internal server error' });
  }
});

module.exports = router;
