import BackgroundTasks
import CoreML
import Darwin
import Foundation
import Metal
import Network
import UIKit
import Vision

struct RunnerReport: Encodable {
  struct MetalGPU: Encodable {
    let name: String
    let unifiedMemory: Bool
    let maxBufferBytes: Int
    let recommendedWorkingSetBytes: UInt64?

    init(_ gpu: MTLDevice) {
      name = gpu.name
      unifiedMemory = gpu.hasUnifiedMemory
      maxBufferBytes = gpu.maxBufferLength
      if #available(iOS 16.0, *) {
        let recommendation = gpu.recommendedMaxWorkingSetSize
        recommendedWorkingSetBytes = recommendation > 0 ? recommendation : nil
      } else {
        recommendedWorkingSetBytes = nil
      }
    }
  }

  let schemaVersion = 1
  let platform = "ios"
  let simulated: Bool
  let observedAt: String
  let osVersion: String
  let logicalCPUCount: Int
  let activeCPUCount: Int
  let physicalMemoryBytes: UInt64
  let availableProcessBytes: Int?
  let availableStorageBytes: Int?
  let charging: String
  let thermal: String
  let lowPowerMode: Bool
  let foreground: Bool
  let network: RunnerNetwork
  let metal: MetalGPU?
  let visionTextRevisions: [Int]
  let coreMLDevices: [String]
  let backgroundGPUResource: Bool?

  @MainActor init(path: NWPath) {
    let process = ProcessInfo.processInfo
    let device = UIDevice.current
    observedAt = ISO8601DateFormatter().string(from: Date())
    osVersion = device.systemVersion
    logicalCPUCount = process.processorCount
    activeCPUCount = process.activeProcessorCount
    physicalMemoryBytes = process.physicalMemory
    #if targetEnvironment(simulator)
    simulated = true
    availableProcessBytes = nil
    #else
    simulated = false
    availableProcessBytes = os_proc_available_memory()
    #endif
    availableStorageBytes = Self.availableStorage()
    charging = Self.charging(device)
    thermal = Self.thermal(process.thermalState)
    lowPowerMode = process.isLowPowerModeEnabled
    foreground = UIApplication.shared.applicationState == .active
    network = RunnerNetwork(path)
    metal = MTLCreateSystemDefaultDevice().map(MetalGPU.init)
    visionTextRevisions = Array(VNRecognizeTextRequest.supportedRevisions)
    coreMLDevices = Self.coreMLDevices()
    if #available(iOS 26.0, *) {
      backgroundGPUResource = BGTaskScheduler.supportedResources.contains(.gpu)
    } else {
      backgroundGPUResource = nil
    }
  }

  private static func availableStorage() -> Int? {
    let home = URL(fileURLWithPath: NSHomeDirectory(), isDirectory: true)
    return (try? home.resourceValues(forKeys: [.volumeAvailableCapacityKey]))?.volumeAvailableCapacity
  }

  @MainActor private static func charging(_ device: UIDevice) -> String {
    let wasMonitoring = device.isBatteryMonitoringEnabled
    device.isBatteryMonitoringEnabled = true
    defer { device.isBatteryMonitoringEnabled = wasMonitoring }
    switch device.batteryState {
    case .unknown: return "unknown"
    case .unplugged: return "unplugged"
    case .charging: return "charging"
    case .full: return "full"
    @unknown default: return "unknown"
    }
  }

  private static func thermal(_ state: ProcessInfo.ThermalState) -> String {
    switch state {
    case .nominal: return "nominal"
    case .fair: return "fair"
    case .serious: return "serious"
    case .critical: return "critical"
    @unknown default: return "unknown"
    }
  }

  private static func coreMLDevices() -> [String] {
    guard #available(iOS 17.0, *) else { return [] }
    return MLModel.availableComputeDevices.map { device in
      switch device {
      case .cpu: return "cpu"
      case .gpu: return "gpu"
      case .neuralEngine: return "neuralEngine"
      @unknown default: return "unknown"
      }
    }.sorted()
  }
}

struct RunnerNetwork: Encodable {
  let status: String
  let interfaces: [String]
  let expensive: Bool
  let constrained: Bool

  init(_ path: NWPath) {
    switch path.status {
    case .satisfied: status = "satisfied"
    case .unsatisfied: status = "unsatisfied"
    case .requiresConnection: status = "requiresConnection"
    @unknown default: status = "unknown"
    }
    let types: [(NWInterface.InterfaceType, String)] = [
      (.wifi, "wifi"), (.cellular, "cellular"), (.wiredEthernet, "wired"), (.other, "other")
    ]
    interfaces = types.filter { path.usesInterfaceType($0.0) }.map(\.1)
    expensive = path.isExpensive
    constrained = path.isConstrained
  }
}
