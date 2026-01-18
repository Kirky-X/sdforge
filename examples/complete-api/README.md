# SDForge Complete API Example

This example demonstrates the full capabilities of SDForge, including:

- ✅ **Multi-protocol support** (HTTP + MCP)
- ✅ **Security features** (authentication, authorization)
- ✅ **Modular organization** (service modules)
- ✅ **Error handling** (custom error types)
- ✅ **Data management** (shared state)
- ✅ **Health checks** (system monitoring)

## 🚀 Quick Start

### 1. Run the Example

```bash
cd examples/complete-api
cargo run --features full
```

### 2. Test the API

#### Health Check
```bash
curl http://localhost:3000/system/api/v1/health
```

#### Authentication
```bash
# Login
curl -X POST http://localhost:3000/auth/api/v1/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "password"}'

# The response will contain a token for authenticated requests
```

#### User Management
```bash
# List all users
curl http://localhost:3000/users/api/v1/

# Get specific user
curl http://localhost:3000/users/api/v1/1

# Create new user
curl -X POST http://localhost:3000/users/api/v1/ \
  -H "Content-Type: application/json" \
  -d '{"username": "newuser", "email": "newuser@example.com"}'
```

## 📁 Project Structure

```
complete-api/
├── Cargo.toml          # Dependencies with full features
├── README.md           # This file
└── src/
    └── main.rs         # Complete API implementation
```

## 🔧 Features Demonstrated

### **Authentication & Authorization**
- Login/logout endpoints
- Token-based authentication
- Permission-based access control

### **Modular Design**
- Separate modules for different domains
- Consistent API structure
- Shared state management

### **Error Handling**
- Custom error types
- Proper HTTP status codes
- User-friendly error messages

### **Data Management**
- In-memory data store
- CRUD operations
- Type-safe data models

## 🎯 Learning Objectives

After studying this example, you'll understand:

1. **How to structure a complete SDForge application**
2. **Best practices for authentication and authorization**
3. **How to organize code with service modules**
4. **Error handling patterns**
5. **Shared state management**

## 🔍 Code Walkthrough

### **Data Models**
- `User`: Represents a user entity
- `CreateUserRequest`: Request DTO for user creation
- `LoginRequest`: Authentication request
- `Token`: Authentication token
- `HealthStatus`: System health information

### **Service Modules**
- `/auth`: Authentication endpoints
- `/users`: User management endpoints  
- `/system`: System information endpoints

### **Shared State**
- `DataStore`: Thread-safe in-memory data store
- Demonstrates dependency injection pattern

## 🚀 Next Steps

1. **Extend the API**: Add more endpoints and features
2. **Add persistence**: Replace in-memory store with database
3. **Add validation**: Implement request validation
4. **Add tests**: Write unit and integration tests
5. **Add documentation**: Generate OpenAPI specs

## 📚 Related Documentation

- [SDForge Main Documentation](../../README.md)
- [Security Configuration](../../README.md#security-configuration)
- [Performance Optimization](../../README.md#performance-optimization)
- [Deployment Guide](../../README.md#deployment-guide)
