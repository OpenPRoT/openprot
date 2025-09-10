## Architecture Description

This diagram illustrates the **Partitioned Resource Architecture** for MCTP integration in Hubris, demonstrating how Platform Root of Trust (PRoT) systems can achieve optimal performance and security through selective resource management strategies.

### **System Overview**

The architecture is divided into two distinct domains:

1. **MCTP Domain** - High-performance, security-critical communication using direct hardware ownership
2. **General Purpose I2C Domain** - Server-based resource management for non-transport functionality

### **Key Motivations**

The partitioned approach addresses critical reliability and performance concerns in shared I2C environments:

- **Blast Radius Limitation**: I2C failures in one domain (e.g., a stuck sensor) cannot impact the other domain's operations
- **Blocking Prevention**: Eliminates scenarios where security-critical MCTP tasks could be blocked waiting for I2C server tasks that are servicing slow or unresponsive devices in the general-purpose domain
- **IPC Complexity Reduction**: A unified I2C server handling both master operations (sensors, PMBus) and slave instances (MCTP endpoints) would require complex IPC protocols to manage bidirectional communication and varying response patterns
- **Fault Isolation**: Hardware or software failures in sensor management cannot compromise MCTP security protocols

### **External Components**

**BMC/Management Controller** (Orange)
- Represents the external Baseboard Management Controller
- Communicates with the Hubris PRoT system via I2C MCTP protocol
- Connects to both primary and backup MCTP buses for redundancy

### **MCTP Domain Components**

#### **Application Layer** (Blue)
- **PLDM Task**: Handles Platform Level Data Model operations including firmware updates, sensor readings, and platform management
- **SPDM Task**: Manages Security Protocol and Data Model for attestation, measurement, and secure communication
- **Vendor Task**: Processes vendor-specific MCTP protocols and custom management functions

#### **MCTP Protocol Layer** (Blue)
- **MCTP Control Task**: 
  - Manages MCTP endpoint discovery and configuration
  - Handles MCTP control messages (Set/Get Endpoint ID, etc.)
  - Software-only task with no direct hardware ownership
  - Communicates with MCTP Router Task via IPC

- **MCTP Router Task**:
  - **Primary responsibility**: Direct ownership of I2C controllers 2 and 3
  - **Transport layer**: Handles raw I2C MCTP packet transmission and reception
  - **Protocol layer**: Routes incoming MCTP messages to appropriate application tasks
  - **Performance critical**: Optimized for low-latency real-time security protocols

#### **MCTP Transport Layer** (Green)
- **I2C Controller 2**: Dedicated MCTP Bus A - primary secure communication channel
- **I2C Controller 3**: Dedicated MCTP Bus B - backup/redundant secure communication channel
- Both controllers are exclusively owned by the MCTP Router Task

*Note: Controller numbers (2, 3, 4, 7) are examples and will vary based on specific hardware implementations.*

### **General Purpose I2C Domain Components**

#### **General Purpose I2C Layer** (Purple)
- **I2C Server Task**: 
  - Provides server-based resource management via IPC
  - Manages multiple I2C controllers for backward compatibility
  - Provides IPC-based access to I2C resources

- **I2C Hardware Controllers**:
  - **I2C Controller 4**: Sensors and PMBus devices
  - **I2C Controller 7**: FRU (Field Replaceable Unit) and expansion interfaces

#### **Sensor Management** (Purple)
- **Sensor Manager**: Collects environmental and system sensor data
- **Thermal Monitor**: Manages thermal policies and fan control
- Both tasks access I2C hardware through the I2C Server Task via IPC

### **Communication Flows**

#### **MCTP Communication Flow** (Dotted lines)
1. BMC initiates I2C MCTP communication on primary bus (I2C Controller 2)
2. Backup communication available on secondary bus (I2C Controller 3)
3. MCTP Router Task receives raw I2C packets directly from hardware

#### **MCTP Protocol Flow** (Solid arrows)
1. **Raw packet processing**: MCTP Router Task processes incoming I2C packets
2. **Message routing**: Router determines destination based on MCTP message type
3. **Application delivery**: Routed messages delivered to PLDM, SPDM, or Vendor tasks
4. **Control message handling**: MCTP control messages routed to MCTP Control Task

#### **General Purpose I2C Flow** (Solid arrows)
1. **IPC requests**: Sensor Manager and Thermal Monitor send I2C requests via IPC
2. **Server processing**: I2C Server Task handles requests and accesses hardware
3. **Hardware operation**: Server controls I2C Controllers 4 and 7 for sensor operations

### **Key Architectural Benefits**

#### **Performance Optimization**
- **Direct hardware access** for MCTP eliminates IPC overhead 
- **Dedicated buses** prevent contention between security-critical MCTP and routine sensor traffic
- **Hardware redundancy** with dual MCTP buses ensures communication reliability

#### **Security Isolation**
- **Complete separation** between MCTP security domain and general purpose I2C operations
- **No shared resources** between security-critical and general-purpose functions
- **Attack surface reduction** through hardware-enforced boundaries

#### **Maintainability**
- **Backward compatibility** preserved for existing I2C sensor infrastructure
- **Clear separation of concerns** between performance-critical and general-purpose operations
- **Incremental deployment** allows gradual migration strategies

#### **Resource Efficiency**
- **Optimal resource allocation** based on performance and security requirements
- **Reuse of existing infrastructure** for non-critical operations
- **Scalable architecture** supporting future protocol additions

### **Trade-offs and Considerations**

#### **Implementation Considerations**
- **Additional tasks**: More tasks required compared to unified I2C server approach, but each with simpler, focused responsibilities
- **Resource allocation**: I2C controllers need to be assigned to appropriate domains during system design (straightforward with abundant controllers in server SoCs and Hubris's static resource allocation at compile time)
- **Separate codepaths**: MCTP and general-purpose I2C operations use different patterns, but this enables domain-specific optimizations

#### **Reduced Flexibility**
- **Static partitioning**: I2C controllers dedicated to MCTP domain cannot be repurposed for other uses (though this is typically not a concern given the abundance of I2C controllers in server-class SoCs)
- **Hardware dependencies**: Architecture requires sufficient I2C controllers to support domain separation (server-class SoCs for datacenter and management applications typically provide 8-16 I2C controllers, making this requirement easily satisfied)

#### **Implementation Challenges**
- **Task priorities**: Must carefully configure task priorities to ensure MCTP Router Task can preempt when necessary
- **Error handling**: Direct hardware ownership requires robust error recovery mechanisms in MCTP Router Task
- **Testing complexity**: Need separate test strategies for both direct ownership and server-based patterns

#### **When This Architecture May Not Be Suitable**
- **Simple deployments**: Systems with minimal I2C traffic may not benefit from the added complexity
- **Highly dynamic requirements**: Applications needing frequent reassignment of I2C resources between functions

This partitioned approach represents the optimal balance between performance, security, and implementation complexity for MCTP integration in Hubris-based PRoT systems.

### **Glossary**

**BMC (Baseboard Management Controller)** - A specialized microcontroller that manages the interface between system management software and platform hardware.

**FRU (Field Replaceable Unit)** - A circuit board, part, or assembly that can be quickly and easily removed from a computer or other piece of electronic equipment and replaced by the user or technician.

**Hubris** - A microkernel-based operating system designed for deeply embedded systems, emphasizing memory safety and deterministic behavior.

**IPC (Inter-Process Communication)** - Mechanisms that allow processes or tasks to communicate and synchronize with each other.

**MCTP (Management Component Transport Protocol)** - A protocol for communication between management controllers and managed devices, designed to be transport-agnostic.

**PLDM (Platform Level Data Model)** - A specification that defines data formats and commands for platform management operations like firmware updates, sensor monitoring, and inventory management.

**PMBus** - A power management protocol that uses I2C/SMBus for communication with power management devices.

**PRoT (Platform Root of Trust)** - A computing engine capable of making attestations about the platform's integrity and identity.

**SPDM (Security Protocol and Data Model)** - A protocol for device authentication, measurement, and secure communication in platform management scenarios.

![MCTP Architecture Diagram](../images/mctp-i2c-domains.png)

