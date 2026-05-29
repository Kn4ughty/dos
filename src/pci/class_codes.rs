use crate::tryfrom::{tryfrom, tryfrom2arg};

tryfrom2arg! {
    #[non_exhaustive]
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum ClassCode {
        Unclassified(UnclassifiedSubClass) = 0x0,
        MassStorageController(MassStorageControllerSubClass) = 0x1,
        NetworkController(NetworkControllerSubClass) = 0x2,
        DisplayController(DisplayControllerSubClass) = 0x3,
        MultimediaController(MultimediaControllerSubClass) = 0x4,
        MemoryController(MemoryControllerSubClass) = 0x5,
        Bridge(BridgeSubClass) = 0x6,
        SimpleCommunicationController(SimpleCommunicationControllerSubClass) = 0x7,
        BaseSystemPerippheral(BaseSystemPeripheralSubClass) = 0x8,
        InputDeviceController(InputDeviceControllerSubClass) = 0x9,
        DockingStation(DockingStationSubClass) = 0xa,
        // Multiple Cpu's?
        Processer(ProcessorSubClass) = 0xb,
        SerialBusController(SerialBusControllerSubClass) = 0xc,
        // As in it controls wireless things, like bluetooth. Not as in the controller is wireless
        WirelessController(WirelessControllerSubClass) = 0xd,
        IntelligentController(InputDeviceControllerSubClass) = 0xe,
        SatelliteCommunication(SatelliteCommunicationControllerSubClass) = 0xf,
        EncryptionController(EncryptionControllerSubClass) = 0x10,
        SignalProcessingController(SignalProcessingControllerSubClass) = 0x11,
        ProcesserAccelerator(NoSubClass) = 0x12,
        UnassignedClassVendor(NoSubClass) = 0xFF,
    },
    u8
}

tryfrom! {
    #[non_exhaustive]
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum UnclassifiedSubClass {
        NonVgaCompatible = 0x00,
        VgaCompatible = 0x01,
    },
    u8
}

tryfrom! {
    #[non_exhaustive]
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum MassStorageControllerSubClass {
        ScsiBus = 0x00,
        Ide = 0x01,
        FloppyDisk = 0x02,
        IpiBus = 0x03,
        Raid = 0x04,
        Ata = 0x05,
        SerialAta = 0x06,
        SerialAttachedScsi = 0x07,
        NonVolatileMemory = 0x08,
        Other = 0x80,
    },
    u8
}

tryfrom! {
    #[non_exhaustive]
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum NetworkControllerSubClass {
        Ethernet = 0x00,
        // Literally what are any of these lol. My dad would know
        TokenRing = 0x01,
        Fddi = 0x02,
        Atm = 0x03,
        Isdn = 0x04,
        WorldFip = 0x05,
        PicmgMultiComputing = 0x06,
        Infiniband = 0x07,
        Fabric = 0x08,
        Other = 0x80,
    },
    u8
}

tryfrom! {
    #[non_exhaustive]
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum DisplayControllerSubClass {
        VgaCompatible = 0x00,
        Xga = 0x01,
        ThreeD = 0x02,
        Other = 0x80,
    },
    u8
}

tryfrom! {
    #[non_exhaustive]
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum MultimediaControllerSubClass {
        Video = 0x00,
        Audio = 0x01,
        ComputerTelephony = 0x02,
        AudioDevice = 0x03,
        Other = 0x80,
    },
    u8
}

tryfrom! {
    #[non_exhaustive]
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum MemoryControllerSubClass {
        Ram = 0x00,
        Flash = 0x01,
        Other = 0x80,
    },
    u8
}

tryfrom! {
    #[non_exhaustive]
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum BridgeSubClass {
        Host = 0x00,
        Isa = 0x01,
        Eisa = 0x02,
        Mca = 0x03,
        PciToPci = 0x04,
        Pcmcia = 0x05,
        NuBus = 0x06,
        CardBus = 0x07,
        RaceWay = 0x08,
        SemiTransparentPciToPci = 0x09,
        InfiniBandToPciHost = 0x0A,
        Other = 0x80,
    },
    u8
}

tryfrom! {
    #[non_exhaustive]
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum SimpleCommunicationControllerSubClass {
        Serial = 0x00,
        Parallel = 0x01,
        MultiportSerial = 0x02,
        Modem = 0x03,
        Gpib = 0x04,
        SmartCard = 0x05,
        Other = 0x80,
    },
    u8
}

tryfrom! {
    #[non_exhaustive]
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum BaseSystemPeripheralSubClass {
        Pic = 0x00,
        Dma = 0x01,
        Timer = 0x02,
        Rtc = 0x03,
        PciHotPlug = 0x04,
        SdHost = 0x05,
        Iommu = 0x06,
        Other = 0x80,
    },
    u8
}

tryfrom! {
    #[non_exhaustive]
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum InputDeviceControllerSubClass {
        Keyboard = 0x00,
        DigitizerPen = 0x01,
        Mouse = 0x02,
        Scanner = 0x03,
        Gameport = 0x04,
        Other = 0x80,
    },
    u8
}

tryfrom! {
    #[non_exhaustive]
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum DockingStationSubClass {
        Generic = 0x00,
        Other = 0x80,
    },
    u8
}

tryfrom! {
    #[non_exhaustive]
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum ProcessorSubClass {
        Intel386 = 0x00,
        Intel486 = 0x01,
        Pentium = 0x02,
        PentiumPro = 0x03,
        Alpha = 0x10,
        PowerPc = 0x20,
        Mips = 0x30,
        CoProcessor = 0x40,
        Other = 0x80,
    },
    u8
}

tryfrom! {
    #[non_exhaustive]
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum SerialBusControllerSubClass {
        FireWire = 0x00,
        AccessBus = 0x01,
        Ssa = 0x02,
        Usb = 0x03,
        FibreChannel = 0x04,
        SmBus = 0x05,
        InfiniBand = 0x06,
        Ipmi = 0x07,
        Sercos = 0x08,
        CanBus = 0x09,
        Other = 0x80,
    },
    u8
}

tryfrom! {
    #[non_exhaustive]
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum WirelessControllerSubClass {
        Irda = 0x00,
        ConsumerIr = 0x01,
        Rf = 0x10,
        Bluetooth = 0x11,
        Broadband = 0x12,
        Ethernet802_1a = 0x20,
        Ethernet802_1b = 0x21,
        Other = 0x80,
    },
    u8
}

tryfrom! {
    #[non_exhaustive]
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum SatelliteCommunicationControllerSubClass {
        Tv = 0x01,
        Audio = 0x02,
        Voice = 0x03,
        Data = 0x04,
    },
    u8
}

tryfrom! {
    #[non_exhaustive]
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum EncryptionControllerSubClass {
        NetworkAndComputing = 0x00,
        Entertainment = 0x10,
        Other = 0x80,
    },
    u8
}

tryfrom! {
    #[non_exhaustive]
    #[repr(u8)]
    #[derive(Debug, PartialEq, Eq)]
    pub enum SignalProcessingControllerSubClass {
        DpioModules = 0x00,
        PerformanceCounters = 0x01,
        CommunicationSynchronizer = 0x10,
        SignalProcessingManagement = 0x20,
        Other = 0x80,
    },
    u8
}

#[derive(Debug, PartialEq, Eq)]
pub enum NoSubClass {}

// This is so stupid
impl TryFrom<u8> for NoSubClass {
    type Error = u8;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Err(value)
    }
}
