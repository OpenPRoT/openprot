Propsal:
Summary 
Use OpenProt as Firmware Device to execute extern staging flow listed below.  

Leverage pldm-lib/pldm-common to build small Update Agent client.
UA Client will stash the image in staging flash
Update Agent will go through discovery of Type5 support via Type 0 commands
Update Agent will go through Inventory via Type 5 commands
Update Agent will trigger out-of-transport image update via ActivatePendingComponentImage of UA component

:::mermaid
sequenceDiagram
    autonumber
    participant UA as UA
    participant OpenProt as OpenProt
    participant fwspi as UA SPI bus (fwspi)

    UA->>OpenProt: GetPldmTypes
    OpenProt-->>UA: PldmTypes 0 and 5 Returned

    UA->>OpenProt: GetPldmCommands Type 5
    OpenProt-->>UA: Commands including ActivatePendingComponent

    UA->>OpenProt: GetFirmwareParameters
    OpenProt-->>UA: FirmwareParameters including ComponentActivationMethods.ActivatePendingImage

    opt Optional query
        UA->>OpenProt: QueryDeviceIdentifiers
        OpenProt-->>UA: Descriptors
    end

    UA->>OpenProt: ActivatePendingComponentImage(AST2070 Component Identifier)
    OpenProt-->>UA: EstimatedTimeForActivation

    Note over OpenProt,UA: OpenProt disables access by UA (notify UA to shutdown, then pull power)

    OpenProt->>fwspi: Claim mastership
    OpenProt->>OpenProt: Verify UA image in staging area

    alt Image good
        OpenProt->>OpenProt: Copy staging image to "B" partition
        OpenProt->>OpenProt: Verify good copy of "B"
        OpenProt->>OpenProt: Erase staging area
        OpenProt->>OpenProt: Mark "B" partition as active
    else Image bad
        OpenProt-->>UA: Activation failed (image invalid)
        Note over UA,OpenProt: Abort update and restore previous state
    end

    OpenProt->>fwspi: Return mastership
    OpenProt-->>UA: Re-enable access and allow UA to boot
    UA->>UA: Boot

    alt UA boot completes
        Note over UA,OpenProt: Good update completed
        Note over OpenProt: Update inactive "A" to match "B" (requires mastership again)
    else Boot fails
        Note over UA,OpenProt: Recovery required
    end
:::
