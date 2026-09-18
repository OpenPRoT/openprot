Propsal:
Build a small Update Agent leverage pldm-lib/pldm-common to run ontop of OpenBMC on AST2700.
Use OpenProt as Firmware Device to execute extern staging flow listed below.  
Update agent will send GetFirmParameters for Inventory and ActivatePendingComponentImage to signal to OpenProt to begin this flow.

sequenceDiagram
    autonumber
    participant BMC as BMC
    participant OpenProt as OpenProt
    participant fwspi as BMC SPI bus (fwspi)

    BMC->>OpenProt: GetFirmwareParameters
    OpenProt-->>BMC: FirmwareParameters

    opt Optional query
        BMC->>OpenProt: QueryDeviceIdentifiers
        OpenProt-->>BMC: Descriptors
    end

    BMC->>OpenProt: ActivatePendingComponentImage(AST2070 Component Identifier)
    OpenProt-->>BMC: EstimatedTimeForActivation

    Note over OpenProt,BMC: OpenProt disables access by BMC (notify BMC to shutdown, then pull power)

    OpenProt->>fwspi: Claim mastership
    OpenProt->>OpenProt: Verify BMC image in staging area

    alt Image good
        OpenProt->>OpenProt: Copy staging image to "B" partition
        OpenProt->>OpenProt: Verify good copy of "B"
        OpenProt->>OpenProt: Erase staging area
        OpenProt->>OpenProt: Mark "B" partition as active
    else Image bad
        OpenProt-->>BMC: Activation failed (image invalid)
        Note over BMC,OpenProt: Abort update and restore previous state
    end

    OpenProt->>fwspi: Return mastership
    OpenProt-->>BMC: Re-enable access and allow BMC to boot
    BMC->>BMC: Boot

    alt BMC boot completes
        Note over BMC,OpenProt: Good update completed
        Note over OpenProt: Update inactive "A" to match "B" (requires mastership again)
    else Boot fails
        Note over BMC,OpenProt: Recovery required
    end
