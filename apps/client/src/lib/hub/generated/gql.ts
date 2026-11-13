/* eslint-disable */
import * as types from './graphql';
import type { TypedDocumentNode as DocumentNode } from '@graphql-typed-document-node/core';

/**
 * Map of all GraphQL operations in the project.
 *
 * This map has several performance disadvantages:
 * 1. It is not tree-shakeable, so it will include all operations in the project.
 * 2. It is not minifiable, so the string of a GraphQL query will be multiple times inside the bundle.
 * 3. It does not support dead code elimination, so it will add unused operations.
 *
 * Therefore it is highly recommended to use the babel or swc plugin for production.
 * Learn more about it here: https://the-guild.dev/graphql/codegen/plugins/presets/preset-client#reducing-bundle-size
 */
type Documents = {
    "\n  mutation Login($input: LoginInput!) {\n    login(input: $input) {\n      token\n    }\n  }\n": typeof types.LoginDocument,
    "\n  mutation SignInBrowser($input: LoginInput!) {\n    signInBrowser(input: $input) {\n      expiresAt\n    }\n  }\n": typeof types.SignInBrowserDocument,
    "\n  mutation Logout {\n    logout\n  }\n": typeof types.LogoutDocument,
    "\n  query Me {\n    me {\n      username\n    }\n  }\n": typeof types.MeDocument,
    "\n  query ApiKeys {\n    apiKeys {\n      id\n      name\n      scope\n      prefix\n      expiresAt\n      createdAt\n      lastUsedAt\n    }\n  }\n": typeof types.ApiKeysDocument,
    "\n  mutation CreateApiKey($input: ApiKeyInput!) {\n    createApiKey(input: $input) {\n      key {\n        id\n        name\n        scope\n        prefix\n        expiresAt\n        createdAt\n        lastUsedAt\n      }\n      secret\n    }\n  }\n": typeof types.CreateApiKeyDocument,
    "\n  mutation RevokeApiKey($id: ID!) {\n    revokeApiKey(id: $id)\n  }\n": typeof types.RevokeApiKeyDocument,
    "\n  query Builders {\n    builders {\n      id\n      name\n      platform {\n        os\n        arch\n      }\n      online\n    }\n  }\n": typeof types.BuildersDocument,
    "\n  mutation EnrolBuilder($name: String!, $platform: BuilderPlatformInput!) {\n    enrolBuilder(name: $name, platform: $platform) {\n      builder {\n        id\n      }\n      token\n    }\n  }\n": typeof types.EnrolBuilderDocument,
    "\n  mutation RevokeBuilder($id: ID!) {\n    revokeBuilder(id: $id)\n  }\n": typeof types.RevokeBuilderDocument,
    "\n  mutation ClaimBuildJob($platform: BuilderPlatformInput!) {\n    claimBuildJob(platform: $platform) {\n      id\n      target\n      version\n      commit\n    }\n  }\n": typeof types.ClaimBuildJobDocument,
    "\n  mutation ReportBuildJob($id: ID!, $state: BuildJobState, $logLines: [String!]!, $failureReason: String) {\n    reportBuildJob(id: $id, state: $state, logLines: $logLines, failureReason: $failureReason) {\n      id\n      state\n    }\n  }\n": typeof types.ReportBuildJobDocument,
    "\n  mutation BuildUploadUrls($id: ID!, $files: [BuildFileInput!]!) {\n    buildUploadUrls(id: $id, files: $files) {\n      name\n      url\n      headers {\n        name\n        value\n      }\n      expiresAt\n    }\n  }\n": typeof types.BuildUploadUrlsDocument,
    "\n  mutation ReportBuildUploaded($id: ID!, $files: [BuildFileInput!]!, $feed: String!) {\n    reportBuildUploaded(id: $id, files: $files, feed: $feed) {\n      id\n      state\n    }\n  }\n": typeof types.ReportBuildUploadedDocument,
    "\n  query Releases($first: Int) {\n    releases(first: $first) {\n      edges {\n        node {\n          id\n          version\n          commit\n          createdAt\n          createdBy\n          jobs {\n            id\n            releaseId\n            target\n            state\n            failureReason\n            attempts\n            builderName\n            stateChangedAt\n          }\n        }\n      }\n    }\n  }\n": typeof types.ReleasesDocument,
    "\n  query NextReleaseVersion {\n    nextReleaseVersion\n  }\n": typeof types.NextReleaseVersionDocument,
    "\n  mutation CreateRelease($version: String!, $commit: String!, $targets: [BuildTarget!]!) {\n    createRelease(version: $version, commit: $commit, targets: $targets) {\n      id\n    }\n  }\n": typeof types.CreateReleaseDocument,
    "\n  mutation CancelBuildJob($id: ID!) {\n    cancelBuildJob(id: $id) {\n      id\n      state\n    }\n  }\n": typeof types.CancelBuildJobDocument,
    "\n  query BuildJobLog($id: ID!, $after: Int, $first: Int) {\n    buildJobLog(id: $id, after: $after, first: $first) {\n      sequence\n      text\n    }\n  }\n": typeof types.BuildJobLogDocument,
    "\n  subscription BuildJobUpdated($jobId: ID) {\n    buildJobUpdated(jobId: $jobId) {\n      job {\n        id\n        releaseId\n        target\n        state\n        failureReason\n        attempts\n        builderName\n        stateChangedAt\n      }\n      logLines {\n        sequence\n        text\n      }\n    }\n  }\n": typeof types.BuildJobUpdatedDocument,
    "\n  subscription HeartbeatAdded {\n    heartbeatAdded(deviceId: null) {\n      checkId\n      deviceId\n      heartbeat {\n        at\n        status\n        responseTimeMs\n      }\n    }\n  }\n": typeof types.HeartbeatAddedDocument,
    "\n  fragment CheckFields on Check {\n    id\n    parent {\n      id\n    }\n    type\n    name\n    intervalSeconds\n    retries\n    retryIntervalSeconds\n    degradedThresholdMs\n    invert\n    paused\n    status\n    certificateDaysLeft\n    domainDaysLeft\n    lastHeartbeat {\n      at\n      status\n      responseTimeMs\n      message\n    }\n    config {\n      __typename\n      ... on IcmpConfig {\n        timeoutMs\n        ipVersion\n      }\n      ... on TcpConfig {\n        port\n        timeoutMs\n      }\n      ... on TlsConfig {\n        port\n        serverName\n        timeoutMs\n      }\n      ... on HttpConfig {\n        url\n        method\n        headers {\n          name\n          value\n        }\n        body\n        maxRedirects\n        acceptedStatus {\n          from\n          to\n        }\n        keyword\n        jsonQuery\n        jsonExpected\n        authKind\n        verifyTls\n        timeoutMs\n      }\n      ... on DnsConfig {\n        name\n        recordType\n        resolver\n        expected\n      }\n      ... on PushConfig {\n        pushPath\n      }\n      ... on GroupConfig {\n        children {\n          id\n          name\n        }\n      }\n      ... on DomainExpiryConfig {\n        domain\n        warnDays\n        timeoutMs\n      }\n    }\n  }\n": typeof types.CheckFieldsFragmentDoc,
    "\n  fragment DockerCheckConfig on DockerConfig {\n    __typename\n    container\n    timeoutMs\n  }\n": typeof types.DockerCheckConfigFragmentDoc,
    "\n  fragment SystemdCheckConfig on SystemdConfig {\n    __typename\n    unit\n    timeoutMs\n  }\n": typeof types.SystemdCheckConfigFragmentDoc,
    "\n  fragment SnmpCheckConfig on SnmpConfig {\n    __typename\n    port\n    version\n    oid\n    expected\n    username\n    timeoutMs\n  }\n": typeof types.SnmpCheckConfigFragmentDoc,
    "\n  fragment TailscalePingCheckConfig on TailscalePingConfig {\n    __typename\n    target\n    timeoutMs\n  }\n": typeof types.TailscalePingCheckConfigFragmentDoc,
    "\n  query CheckDefaults {\n    checkDefaults {\n      intervalSeconds\n      retries\n      retryIntervalSeconds\n      degradedThresholdMs\n    }\n  }\n": typeof types.CheckDefaultsDocument,
    "\n  mutation SetCheckDefaults($input: CheckDefaultsInput!) {\n    setCheckDefaults(input: $input) {\n      intervalSeconds\n      retries\n      retryIntervalSeconds\n      degradedThresholdMs\n    }\n  }\n": typeof types.SetCheckDefaultsDocument,
    "\n  query DeviceChecks($deviceId: ID!) {\n    device(id: $deviceId) {\n      id\n      checks {\n        ...CheckFields\n      }\n    }\n  }\n": typeof types.DeviceChecksDocument,
    "\n  query CheckHeartbeats($id: ID!, $first: Int, $after: String) {\n    check(id: $id) {\n      id\n      heartbeats(first: $first, after: $after) {\n        pageInfo {\n          hasNextPage\n          endCursor\n        }\n        edges {\n          node {\n            at\n            status\n            responseTimeMs\n            message\n          }\n        }\n      }\n    }\n  }\n": typeof types.CheckHeartbeatsDocument,
    "\n  mutation CreateCheck($input: CreateCheckInput!) {\n    createCheck(input: $input) {\n      ...CheckFields\n    }\n  }\n": typeof types.CreateCheckDocument,
    "\n  mutation UpdateCheck($id: ID!, $input: UpdateCheckInput!) {\n    updateCheck(id: $id, input: $input) {\n      ...CheckFields\n    }\n  }\n": typeof types.UpdateCheckDocument,
    "\n  mutation DeleteCheck($id: ID!) {\n    deleteCheck(id: $id)\n  }\n": typeof types.DeleteCheckDocument,
    "\n  mutation SetCheckPaused($id: ID!, $paused: Boolean!) {\n    setCheckPaused(id: $id, paused: $paused) {\n      ...CheckFields\n    }\n  }\n": typeof types.SetCheckPausedDocument,
    "\n  query CheckNotifications($id: ID!) {\n    check(id: $id) {\n      notifications {\n        channels {\n          id\n        }\n        resendEveryBeats\n      }\n    }\n  }\n": typeof types.CheckNotificationsDocument,
    "\n  mutation SetCheckNotifications($checkId: ID!, $input: CheckNotificationsInput!) {\n    setCheckNotifications(checkId: $checkId, input: $input) {\n      channels {\n        id\n      }\n      resendEveryBeats\n    }\n  }\n": typeof types.SetCheckNotificationsDocument,
    "\n  query Devices($filter: DeviceFilter, $sort: DeviceSort, $first: Int, $after: String) {\n    devices(filter: $filter, sort: $sort, first: $first, after: $after) {\n      totalCount\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          name\n          host\n          ip\n          tags\n          maintenance\n          status\n          uptime(window: DAY)\n          os\n          distribution\n          macAddress\n          metricsSource\n          readings {\n            ...CardReadings\n          }\n          accessWarnings\n          webInterfaces {\n            ...WebInterfaceFields\n          }\n          rdp {\n            enabled\n          }\n          vnc {\n            enabled\n            username\n          }\n          ssh {\n            enabled\n            hostKey {\n              fingerprintSha256\n            }\n          }\n          checks {\n            id\n            heartbeats(first: 50) {\n              edges {\n                node {\n                  at\n                  status\n                  responseTimeMs\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  }\n": typeof types.DevicesDocument,
    "\n  query DeviceHosts($after: String) {\n    devices(first: 200, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          host\n        }\n      }\n    }\n  }\n": typeof types.DeviceHostsDocument,
    "\n  query Tags {\n    tags\n  }\n": typeof types.TagsDocument,
    "\n  subscription DeviceStatusChanged {\n    deviceStatusChanged(deviceId: null) {\n      deviceId\n      status\n      at\n    }\n  }\n": typeof types.DeviceStatusChangedDocument,
    "\n  subscription CardReadingsAdded {\n    metricsAdded(deviceId: null) {\n      deviceId\n      readings {\n        ...CardReadings\n      }\n    }\n  }\n": typeof types.CardReadingsAddedDocument,
    "\n  fragment DeviceDetailFields on Device {\n    id\n    name\n    host\n    ip\n    kind\n    os\n    distribution\n    tags\n    maintenance\n    status\n    site {\n      id\n      name\n    }\n    ssh {\n      enabled\n      port\n      username\n      auth\n      credentialStored\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n    }\n    macAddress\n    metricsSource\n    accessWarnings\n    rdp {\n      enabled\n      port\n      username\n      domain\n      security\n      ignoreCertificate\n      resizeMethod\n      audioOut\n      microphone\n      sharedDrive\n      credentialStored\n      serverCertificate {\n        fingerprintSha256\n        subject\n        notAfter\n        firstSeenAt\n      }\n    }\n    vnc {\n      enabled\n      port\n      username\n      credentialStored\n    }\n    transfer {\n      upload\n      download\n      copy\n      paste\n    }\n    webInterfaces {\n      ...WebInterfaceFields\n    }\n    webInterfaceSuggestions {\n      ...WebInterfaceSuggestionFields\n    }\n    day: uptime(window: DAY)\n    week: uptime(window: WEEK)\n    month: uptime(window: MONTH)\n  }\n": typeof types.DeviceDetailFieldsFragmentDoc,
    "\n  fragment CardReadings on Readings {\n    at\n    cpuPercent\n    memoryPercent\n    diskPercent\n    diskMount\n    otherDrives\n    vramPercent\n    vramGpu {\n      index\n      name\n      vendor\n      hasVram\n    }\n  }\n": typeof types.CardReadingsFragmentDoc,
    "\n  query Device($id: ID!) {\n    device(id: $id) {\n      ...DeviceDetailFields\n    }\n  }\n": typeof types.DeviceDocument,
    "\n  query Sites {\n    sites {\n      id\n      name\n      subnet\n    }\n  }\n": typeof types.SitesDocument,
    "\n  mutation CreateSite($name: String!, $subnet: Cidr!) {\n    createSite(name: $name, subnet: $subnet) {\n      id\n      name\n      subnet\n    }\n  }\n": typeof types.CreateSiteDocument,
    "\n  mutation CreateDevice($input: CreateDeviceInput!) {\n    createDevice(input: $input) {\n      ...DeviceDetailFields\n    }\n  }\n": typeof types.CreateDeviceDocument,
    "\n  mutation UpdateDevice($id: ID!, $input: UpdateDeviceInput!) {\n    updateDevice(id: $id, input: $input) {\n      ...DeviceDetailFields\n    }\n  }\n": typeof types.UpdateDeviceDocument,
    "\n  mutation DeleteDevice($id: ID!) {\n    deleteDevice(id: $id)\n  }\n": typeof types.DeleteDeviceDocument,
    "\n  mutation SetSshCredential($deviceId: ID!, $credential: SshCredentialInput!) {\n    setSshCredential(deviceId: $deviceId, credential: $credential) {\n      credentialStored\n    }\n  }\n": typeof types.SetSshCredentialDocument,
    "\n  mutation ClearSshCredential($deviceId: ID!) {\n    clearSshCredential(deviceId: $deviceId) {\n      credentialStored\n    }\n  }\n": typeof types.ClearSshCredentialDocument,
    "\n  mutation ClearRdpCredential($deviceId: ID!) {\n    clearRdpCredential(deviceId: $deviceId) {\n      credentialStored\n    }\n  }\n": typeof types.ClearRdpCredentialDocument,
    "\n  mutation ClearVncCredential($deviceId: ID!) {\n    clearVncCredential(deviceId: $deviceId) {\n      credentialStored\n    }\n  }\n": typeof types.ClearVncCredentialDocument,
    "\n  query DevicePolicy($id: ID!) {\n    device(id: $id) {\n      sessionLimit\n    }\n  }\n": typeof types.DevicePolicyDocument,
    "\n  mutation SetDevicePolicy($id: ID!, $input: UpdateDeviceInput!) {\n    updateDevice(id: $id, input: $input) {\n      sessionLimit\n    }\n  }\n": typeof types.SetDevicePolicyDocument,
    "\n  query Events($filter: EventFilter, $first: Int, $after: String) {\n    events(filter: $filter, first: $first, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          at\n          severity\n          source\n          deviceId\n          deviceName\n          message\n        }\n      }\n    }\n  }\n": typeof types.EventsDocument,
    "\n  subscription EventAdded($minSeverity: Severity) {\n    eventAdded(minSeverity: $minSeverity) {\n      id\n      at\n      severity\n      source\n      deviceId\n      deviceName\n      message\n    }\n  }\n": typeof types.EventAddedDocument,
    "\n  query DeviceMetrics($id: ID!, $range: MetricRange!) {\n    device(id: $id) {\n      id\n      metricsSource\n      gpus {\n        index\n        name\n        vendor\n        hasVram\n      }\n      readings {\n        at\n        cpuPercent\n        memoryPercent\n        diskPercent\n        diskMount\n        otherDrives\n        vramPercent\n        vramGpu {\n          index\n          name\n          vendor\n          hasVram\n        }\n      }\n      storage {\n        drives {\n          name\n          kind\n          sizeBytes\n          mounted\n          mounts {\n            path\n            usedBytes\n            sizeBytes\n            percent\n          }\n          readBps\n          writeBps\n          busyPercent\n        }\n      }\n      metrics(range: $range) {\n        kind\n        label\n        gpuIndex\n        points {\n          at\n          value\n          min\n          max\n        }\n      }\n    }\n  }\n": typeof types.DeviceMetricsDocument,
    "\n  subscription MetricsAdded($deviceId: ID!) {\n    metricsAdded(deviceId: $deviceId) {\n      deviceId\n      at\n      samples {\n        kind\n        label\n        value\n      }\n      readings {\n        at\n        cpuPercent\n        memoryPercent\n        diskPercent\n        diskMount\n        otherDrives\n        vramPercent\n        vramGpu {\n          index\n          name\n          vendor\n          hasVram\n        }\n      }\n    }\n  }\n": typeof types.MetricsAddedDocument,
    "\n  query AgentStatus($id: ID!) {\n    device(id: $id) {\n      id\n      metricsSource\n      metricsStale\n    }\n  }\n": typeof types.AgentStatusDocument,
    "\n  mutation EnrolAgent($deviceId: ID!) {\n    enrolAgent(deviceId: $deviceId) {\n      code\n      expiresAt\n    }\n  }\n": typeof types.EnrolAgentDocument,
    "\n  mutation RevokeAgentToken($deviceId: ID!) {\n    revokeAgentToken(deviceId: $deviceId) {\n      id\n      metricsSource\n      metricsStale\n    }\n  }\n": typeof types.RevokeAgentTokenDocument,
    "\n  fragment DeliveryFields on Delivery {\n    id\n    channelId\n    kind\n    at\n    ok\n    reason\n    code\n  }\n": typeof types.DeliveryFieldsFragmentDoc,
    "\n  fragment NotificationChannelFields on NotificationChannel {\n    id\n    name\n    kind\n    onByDefault\n    settings {\n      __typename\n      ... on WebhookSettings {\n        origin\n        method\n        headerNames\n      }\n      ... on NtfySettings {\n        server\n        priority\n        tokenStored\n      }\n      ... on EmailSettings {\n        host\n        port\n        security\n        username\n        from\n        to\n        passwordStored\n      }\n      ... on AppriseSettings {\n        server\n        mode\n        tags\n      }\n    }\n    template {\n      title\n      body\n    }\n    lastDelivery {\n      ...DeliveryFields\n    }\n  }\n": typeof types.NotificationChannelFieldsFragmentDoc,
    "\n  fragment NotificationSettingsFields on NotificationSettings {\n    certificateDays\n    template {\n      title\n      body\n    }\n    placeholders\n  }\n": typeof types.NotificationSettingsFieldsFragmentDoc,
    "\n  query NotificationChannels {\n    notificationChannels {\n      ...NotificationChannelFields\n    }\n  }\n": typeof types.NotificationChannelsDocument,
    "\n  query NotificationSettings {\n    notificationSettings {\n      ...NotificationSettingsFields\n    }\n  }\n": typeof types.NotificationSettingsDocument,
    "\n  mutation CreateNotificationChannel($input: NotificationChannelInput!) {\n    createNotificationChannel(input: $input) {\n      ...NotificationChannelFields\n    }\n  }\n": typeof types.CreateNotificationChannelDocument,
    "\n  mutation UpdateNotificationChannel($id: ID!, $input: NotificationChannelInput!) {\n    updateNotificationChannel(id: $id, input: $input) {\n      ...NotificationChannelFields\n    }\n  }\n": typeof types.UpdateNotificationChannelDocument,
    "\n  mutation DeleteNotificationChannel($id: ID!) {\n    deleteNotificationChannel(id: $id)\n  }\n": typeof types.DeleteNotificationChannelDocument,
    "\n  mutation SendTestNotification($id: ID!) {\n    sendTestNotification(id: $id) {\n      ...DeliveryFields\n    }\n  }\n": typeof types.SendTestNotificationDocument,
    "\n  mutation SetNotificationSettings($input: NotificationSettingsInput!) {\n    setNotificationSettings(input: $input) {\n      ...NotificationSettingsFields\n    }\n  }\n": typeof types.SetNotificationSettingsDocument,
    "\n  query Retention {\n    retention {\n      subject\n      days\n      minDays\n      maxDays\n      defaultDays\n    }\n  }\n": typeof types.RetentionDocument,
    "\n  mutation SetRetention($subject: RetentionSubject!, $days: Int!) {\n    setRetention(subject: $subject, days: $days) {\n      subject\n      days\n      minDays\n      maxDays\n      defaultDays\n    }\n  }\n": typeof types.SetRetentionDocument,
    "\n  mutation StartSession($input: StartSessionInput!) {\n    startSession(input: $input) {\n      sessionId\n      token\n      tunnelPath\n      expiresAt\n      wakeSent\n    }\n  }\n": typeof types.StartSessionDocument,
    "\n  mutation EndSession($id: ID!) {\n    endSession(id: $id) {\n      id\n      deviceId\n      deviceName\n      protocol\n      startedAt\n      endedAt\n      endReason\n    }\n  }\n": typeof types.EndSessionDocument,
    "\n  mutation TestConnection($input: ConnectionTestInput!) {\n    testConnection(input: $input) {\n      reachable\n      authenticated\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n      hostKeyChanged\n      message\n    }\n  }\n": typeof types.TestConnectionDocument,
    "\n  mutation AcceptHostKey($deviceId: ID!, $fingerprintSha256: String!) {\n    acceptHostKey(deviceId: $deviceId, fingerprintSha256: $fingerprintSha256) {\n      algorithm\n      fingerprintSha256\n      firstSeenAt\n    }\n  }\n": typeof types.AcceptHostKeyDocument,
    "\n  mutation TestRemoteConnection($input: ConnectionTestInput!) {\n    testConnection(input: $input) {\n      reachable\n      authenticated\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n      hostKeyChanged\n      message\n      serverCertificate {\n        fingerprintSha256\n        subject\n        notAfter\n        firstSeenAt\n      }\n      certificateChanged\n      certificateMatched\n    }\n  }\n": typeof types.TestRemoteConnectionDocument,
    "\n  mutation AcceptServerCertificate($deviceId: ID!, $fingerprintSha256: String!) {\n    acceptServerCertificate(deviceId: $deviceId, fingerprintSha256: $fingerprintSha256) {\n      fingerprintSha256\n      subject\n      notAfter\n      firstSeenAt\n    }\n  }\n": typeof types.AcceptServerCertificateDocument,
    "\n  query DeviceTransfer($id: ID!) {\n    device(id: $id) {\n      transfer {\n        copy\n        paste\n      }\n    }\n  }\n": typeof types.DeviceTransferDocument,
    "\n  mutation SetRdpCredential($deviceId: ID!, $credential: PasswordCredentialInput!) {\n    setRdpCredential(deviceId: $deviceId, credential: $credential) {\n      credentialStored\n    }\n  }\n": typeof types.SetRdpCredentialDocument,
    "\n  mutation SetVncCredential($deviceId: ID!, $credential: PasswordCredentialInput!) {\n    setVncCredential(deviceId: $deviceId, credential: $credential) {\n      credentialStored\n    }\n  }\n": typeof types.SetVncCredentialDocument,
    "\n  query DeviceSessions($deviceId: ID!, $first: Int, $after: String) {\n    sessions(deviceId: $deviceId, first: $first, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          protocol\n          operatorName\n          startedAt\n          endedAt\n          endReason\n        }\n      }\n    }\n  }\n": typeof types.DeviceSessionsDocument,
    "\n  mutation OpenFiles($sessionId: ID!) {\n    openFiles(sessionId: $sessionId) {\n      sessionId\n      token\n      tunnelPath\n      expiresAt\n    }\n  }\n": typeof types.OpenFilesDocument,
    "\n  query LiveSessions($first: Int) {\n    sessions(live: true, first: $first) {\n      edges {\n        node {\n          id\n          deviceId\n          deviceName\n          protocol\n          operatorName\n          startedAt\n          clientAddr\n        }\n      }\n    }\n  }\n": typeof types.LiveSessionsDocument,
    "\n  subscription LiveSessionChanged {\n    sessionChanged {\n      session {\n        id\n        deviceId\n        deviceName\n        protocol\n        operatorName\n        startedAt\n        clientAddr\n        endedAt\n        endReason\n      }\n    }\n  }\n": typeof types.LiveSessionChangedDocument,
    "\n  mutation WakeDevice($deviceId: ID!) {\n    wakeDevice(deviceId: $deviceId) {\n      sentAt\n      relayName\n    }\n  }\n": typeof types.WakeDeviceDocument,
    "\n  fragment VpnNodeFields on VpnNode {\n    name\n    address\n    os\n    role\n    tags\n    routes\n    waitingRoutes\n    online\n    lastSeen\n    keyExpiry\n    keyState\n    path\n    relayRegion\n    inAnima\n    device {\n      id\n      name\n    }\n  }\n": typeof types.VpnNodeFieldsFragmentDoc,
    "\n  fragment VpnPictureFields on VpnPicture {\n    readAt\n    fresh\n    hub {\n      ...VpnNodeFields\n    }\n    sites {\n      site {\n        id\n        name\n        subnet\n      }\n      router {\n        ...VpnNodeFields\n      }\n      state\n      path\n      relayRegion\n      hasPingCheck\n      waitingRouter {\n        ...VpnNodeFields\n      }\n    }\n    nodes {\n      ...VpnNodeFields\n    }\n    broker\n  }\n": typeof types.VpnPictureFieldsFragmentDoc,
    "\n  query Vpn {\n    vpn {\n      ...VpnPictureFields\n    }\n  }\n": typeof types.VpnDocument,
    "\n  subscription VpnChanged {\n    vpnChanged {\n      ...VpnPictureFields\n    }\n  }\n": typeof types.VpnChangedDocument,
    "\n  mutation CreateVpnInvite {\n    createVpnInvite {\n      key\n      expiresAt\n      steps\n    }\n  }\n": typeof types.CreateVpnInviteDocument,
    "\n  mutation CreateRouterKey($siteId: ID!) {\n    createRouterKey(siteId: $siteId) {\n      key\n      expiresAt\n      steps\n    }\n  }\n": typeof types.CreateRouterKeyDocument,
    "\n  mutation RegisterVpnNode($authId: String!) {\n    registerVpnNode(authId: $authId) {\n      name\n      address\n      keyExpiry\n    }\n  }\n": typeof types.RegisterVpnNodeDocument,
    "\n  mutation RenewVpnKey($name: String!) {\n    renewVpnKey(name: $name) {\n      name\n      address\n      keyExpiry\n    }\n  }\n": typeof types.RenewVpnKeyDocument,
    "\n  mutation ApproveSiteRoute($siteId: ID!) {\n    approveSiteRoute(siteId: $siteId)\n  }\n": typeof types.ApproveSiteRouteDocument,
    "\n  fragment WebInterfaceSuggestionFields on WebInterfaceSuggestion {\n    checkId\n    url\n    label\n  }\n": typeof types.WebInterfaceSuggestionFieldsFragmentDoc,
    "\n  fragment WebInterfaceFields on WebInterface {\n    id\n    deviceId\n    label\n    url\n    openIn\n    pinnedFingerprintSha256\n    pinnedAt\n  }\n": typeof types.WebInterfaceFieldsFragmentDoc,
    "\n  mutation CreateWebInterface($deviceId: ID!, $input: WebInterfaceInput!) {\n    createWebInterface(deviceId: $deviceId, input: $input) {\n      ...WebInterfaceFields\n    }\n  }\n": typeof types.CreateWebInterfaceDocument,
    "\n  mutation UpdateWebInterface($id: ID!, $input: WebInterfaceInput!) {\n    updateWebInterface(id: $id, input: $input) {\n      ...WebInterfaceFields\n    }\n  }\n": typeof types.UpdateWebInterfaceDocument,
    "\n  mutation DeleteWebInterface($id: ID!) {\n    deleteWebInterface(id: $id)\n  }\n": typeof types.DeleteWebInterfaceDocument,
    "\n  mutation AcceptWebInterfaceSuggestion($checkId: ID!) {\n    acceptWebInterfaceSuggestion(checkId: $checkId) {\n      ...WebInterfaceFields\n    }\n  }\n": typeof types.AcceptWebInterfaceSuggestionDocument,
    "\n  mutation DismissWebInterfaceSuggestion($checkId: ID!) {\n    dismissWebInterfaceSuggestion(checkId: $checkId)\n  }\n": typeof types.DismissWebInterfaceSuggestionDocument,
    "\n  mutation AcceptWebCertificate($id: ID!, $fingerprintSha256: String!) {\n    acceptWebCertificate(id: $id, fingerprintSha256: $fingerprintSha256) {\n      id\n      pinnedFingerprintSha256\n      pinnedAt\n    }\n  }\n": typeof types.AcceptWebCertificateDocument,
};
const documents: Documents = {
    "\n  mutation Login($input: LoginInput!) {\n    login(input: $input) {\n      token\n    }\n  }\n": types.LoginDocument,
    "\n  mutation SignInBrowser($input: LoginInput!) {\n    signInBrowser(input: $input) {\n      expiresAt\n    }\n  }\n": types.SignInBrowserDocument,
    "\n  mutation Logout {\n    logout\n  }\n": types.LogoutDocument,
    "\n  query Me {\n    me {\n      username\n    }\n  }\n": types.MeDocument,
    "\n  query ApiKeys {\n    apiKeys {\n      id\n      name\n      scope\n      prefix\n      expiresAt\n      createdAt\n      lastUsedAt\n    }\n  }\n": types.ApiKeysDocument,
    "\n  mutation CreateApiKey($input: ApiKeyInput!) {\n    createApiKey(input: $input) {\n      key {\n        id\n        name\n        scope\n        prefix\n        expiresAt\n        createdAt\n        lastUsedAt\n      }\n      secret\n    }\n  }\n": types.CreateApiKeyDocument,
    "\n  mutation RevokeApiKey($id: ID!) {\n    revokeApiKey(id: $id)\n  }\n": types.RevokeApiKeyDocument,
    "\n  query Builders {\n    builders {\n      id\n      name\n      platform {\n        os\n        arch\n      }\n      online\n    }\n  }\n": types.BuildersDocument,
    "\n  mutation EnrolBuilder($name: String!, $platform: BuilderPlatformInput!) {\n    enrolBuilder(name: $name, platform: $platform) {\n      builder {\n        id\n      }\n      token\n    }\n  }\n": types.EnrolBuilderDocument,
    "\n  mutation RevokeBuilder($id: ID!) {\n    revokeBuilder(id: $id)\n  }\n": types.RevokeBuilderDocument,
    "\n  mutation ClaimBuildJob($platform: BuilderPlatformInput!) {\n    claimBuildJob(platform: $platform) {\n      id\n      target\n      version\n      commit\n    }\n  }\n": types.ClaimBuildJobDocument,
    "\n  mutation ReportBuildJob($id: ID!, $state: BuildJobState, $logLines: [String!]!, $failureReason: String) {\n    reportBuildJob(id: $id, state: $state, logLines: $logLines, failureReason: $failureReason) {\n      id\n      state\n    }\n  }\n": types.ReportBuildJobDocument,
    "\n  mutation BuildUploadUrls($id: ID!, $files: [BuildFileInput!]!) {\n    buildUploadUrls(id: $id, files: $files) {\n      name\n      url\n      headers {\n        name\n        value\n      }\n      expiresAt\n    }\n  }\n": types.BuildUploadUrlsDocument,
    "\n  mutation ReportBuildUploaded($id: ID!, $files: [BuildFileInput!]!, $feed: String!) {\n    reportBuildUploaded(id: $id, files: $files, feed: $feed) {\n      id\n      state\n    }\n  }\n": types.ReportBuildUploadedDocument,
    "\n  query Releases($first: Int) {\n    releases(first: $first) {\n      edges {\n        node {\n          id\n          version\n          commit\n          createdAt\n          createdBy\n          jobs {\n            id\n            releaseId\n            target\n            state\n            failureReason\n            attempts\n            builderName\n            stateChangedAt\n          }\n        }\n      }\n    }\n  }\n": types.ReleasesDocument,
    "\n  query NextReleaseVersion {\n    nextReleaseVersion\n  }\n": types.NextReleaseVersionDocument,
    "\n  mutation CreateRelease($version: String!, $commit: String!, $targets: [BuildTarget!]!) {\n    createRelease(version: $version, commit: $commit, targets: $targets) {\n      id\n    }\n  }\n": types.CreateReleaseDocument,
    "\n  mutation CancelBuildJob($id: ID!) {\n    cancelBuildJob(id: $id) {\n      id\n      state\n    }\n  }\n": types.CancelBuildJobDocument,
    "\n  query BuildJobLog($id: ID!, $after: Int, $first: Int) {\n    buildJobLog(id: $id, after: $after, first: $first) {\n      sequence\n      text\n    }\n  }\n": types.BuildJobLogDocument,
    "\n  subscription BuildJobUpdated($jobId: ID) {\n    buildJobUpdated(jobId: $jobId) {\n      job {\n        id\n        releaseId\n        target\n        state\n        failureReason\n        attempts\n        builderName\n        stateChangedAt\n      }\n      logLines {\n        sequence\n        text\n      }\n    }\n  }\n": types.BuildJobUpdatedDocument,
    "\n  subscription HeartbeatAdded {\n    heartbeatAdded(deviceId: null) {\n      checkId\n      deviceId\n      heartbeat {\n        at\n        status\n        responseTimeMs\n      }\n    }\n  }\n": types.HeartbeatAddedDocument,
    "\n  fragment CheckFields on Check {\n    id\n    parent {\n      id\n    }\n    type\n    name\n    intervalSeconds\n    retries\n    retryIntervalSeconds\n    degradedThresholdMs\n    invert\n    paused\n    status\n    certificateDaysLeft\n    domainDaysLeft\n    lastHeartbeat {\n      at\n      status\n      responseTimeMs\n      message\n    }\n    config {\n      __typename\n      ... on IcmpConfig {\n        timeoutMs\n        ipVersion\n      }\n      ... on TcpConfig {\n        port\n        timeoutMs\n      }\n      ... on TlsConfig {\n        port\n        serverName\n        timeoutMs\n      }\n      ... on HttpConfig {\n        url\n        method\n        headers {\n          name\n          value\n        }\n        body\n        maxRedirects\n        acceptedStatus {\n          from\n          to\n        }\n        keyword\n        jsonQuery\n        jsonExpected\n        authKind\n        verifyTls\n        timeoutMs\n      }\n      ... on DnsConfig {\n        name\n        recordType\n        resolver\n        expected\n      }\n      ... on PushConfig {\n        pushPath\n      }\n      ... on GroupConfig {\n        children {\n          id\n          name\n        }\n      }\n      ... on DomainExpiryConfig {\n        domain\n        warnDays\n        timeoutMs\n      }\n    }\n  }\n": types.CheckFieldsFragmentDoc,
    "\n  fragment DockerCheckConfig on DockerConfig {\n    __typename\n    container\n    timeoutMs\n  }\n": types.DockerCheckConfigFragmentDoc,
    "\n  fragment SystemdCheckConfig on SystemdConfig {\n    __typename\n    unit\n    timeoutMs\n  }\n": types.SystemdCheckConfigFragmentDoc,
    "\n  fragment SnmpCheckConfig on SnmpConfig {\n    __typename\n    port\n    version\n    oid\n    expected\n    username\n    timeoutMs\n  }\n": types.SnmpCheckConfigFragmentDoc,
    "\n  fragment TailscalePingCheckConfig on TailscalePingConfig {\n    __typename\n    target\n    timeoutMs\n  }\n": types.TailscalePingCheckConfigFragmentDoc,
    "\n  query CheckDefaults {\n    checkDefaults {\n      intervalSeconds\n      retries\n      retryIntervalSeconds\n      degradedThresholdMs\n    }\n  }\n": types.CheckDefaultsDocument,
    "\n  mutation SetCheckDefaults($input: CheckDefaultsInput!) {\n    setCheckDefaults(input: $input) {\n      intervalSeconds\n      retries\n      retryIntervalSeconds\n      degradedThresholdMs\n    }\n  }\n": types.SetCheckDefaultsDocument,
    "\n  query DeviceChecks($deviceId: ID!) {\n    device(id: $deviceId) {\n      id\n      checks {\n        ...CheckFields\n      }\n    }\n  }\n": types.DeviceChecksDocument,
    "\n  query CheckHeartbeats($id: ID!, $first: Int, $after: String) {\n    check(id: $id) {\n      id\n      heartbeats(first: $first, after: $after) {\n        pageInfo {\n          hasNextPage\n          endCursor\n        }\n        edges {\n          node {\n            at\n            status\n            responseTimeMs\n            message\n          }\n        }\n      }\n    }\n  }\n": types.CheckHeartbeatsDocument,
    "\n  mutation CreateCheck($input: CreateCheckInput!) {\n    createCheck(input: $input) {\n      ...CheckFields\n    }\n  }\n": types.CreateCheckDocument,
    "\n  mutation UpdateCheck($id: ID!, $input: UpdateCheckInput!) {\n    updateCheck(id: $id, input: $input) {\n      ...CheckFields\n    }\n  }\n": types.UpdateCheckDocument,
    "\n  mutation DeleteCheck($id: ID!) {\n    deleteCheck(id: $id)\n  }\n": types.DeleteCheckDocument,
    "\n  mutation SetCheckPaused($id: ID!, $paused: Boolean!) {\n    setCheckPaused(id: $id, paused: $paused) {\n      ...CheckFields\n    }\n  }\n": types.SetCheckPausedDocument,
    "\n  query CheckNotifications($id: ID!) {\n    check(id: $id) {\n      notifications {\n        channels {\n          id\n        }\n        resendEveryBeats\n      }\n    }\n  }\n": types.CheckNotificationsDocument,
    "\n  mutation SetCheckNotifications($checkId: ID!, $input: CheckNotificationsInput!) {\n    setCheckNotifications(checkId: $checkId, input: $input) {\n      channels {\n        id\n      }\n      resendEveryBeats\n    }\n  }\n": types.SetCheckNotificationsDocument,
    "\n  query Devices($filter: DeviceFilter, $sort: DeviceSort, $first: Int, $after: String) {\n    devices(filter: $filter, sort: $sort, first: $first, after: $after) {\n      totalCount\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          name\n          host\n          ip\n          tags\n          maintenance\n          status\n          uptime(window: DAY)\n          os\n          distribution\n          macAddress\n          metricsSource\n          readings {\n            ...CardReadings\n          }\n          accessWarnings\n          webInterfaces {\n            ...WebInterfaceFields\n          }\n          rdp {\n            enabled\n          }\n          vnc {\n            enabled\n            username\n          }\n          ssh {\n            enabled\n            hostKey {\n              fingerprintSha256\n            }\n          }\n          checks {\n            id\n            heartbeats(first: 50) {\n              edges {\n                node {\n                  at\n                  status\n                  responseTimeMs\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  }\n": types.DevicesDocument,
    "\n  query DeviceHosts($after: String) {\n    devices(first: 200, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          host\n        }\n      }\n    }\n  }\n": types.DeviceHostsDocument,
    "\n  query Tags {\n    tags\n  }\n": types.TagsDocument,
    "\n  subscription DeviceStatusChanged {\n    deviceStatusChanged(deviceId: null) {\n      deviceId\n      status\n      at\n    }\n  }\n": types.DeviceStatusChangedDocument,
    "\n  subscription CardReadingsAdded {\n    metricsAdded(deviceId: null) {\n      deviceId\n      readings {\n        ...CardReadings\n      }\n    }\n  }\n": types.CardReadingsAddedDocument,
    "\n  fragment DeviceDetailFields on Device {\n    id\n    name\n    host\n    ip\n    kind\n    os\n    distribution\n    tags\n    maintenance\n    status\n    site {\n      id\n      name\n    }\n    ssh {\n      enabled\n      port\n      username\n      auth\n      credentialStored\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n    }\n    macAddress\n    metricsSource\n    accessWarnings\n    rdp {\n      enabled\n      port\n      username\n      domain\n      security\n      ignoreCertificate\n      resizeMethod\n      audioOut\n      microphone\n      sharedDrive\n      credentialStored\n      serverCertificate {\n        fingerprintSha256\n        subject\n        notAfter\n        firstSeenAt\n      }\n    }\n    vnc {\n      enabled\n      port\n      username\n      credentialStored\n    }\n    transfer {\n      upload\n      download\n      copy\n      paste\n    }\n    webInterfaces {\n      ...WebInterfaceFields\n    }\n    webInterfaceSuggestions {\n      ...WebInterfaceSuggestionFields\n    }\n    day: uptime(window: DAY)\n    week: uptime(window: WEEK)\n    month: uptime(window: MONTH)\n  }\n": types.DeviceDetailFieldsFragmentDoc,
    "\n  fragment CardReadings on Readings {\n    at\n    cpuPercent\n    memoryPercent\n    diskPercent\n    diskMount\n    otherDrives\n    vramPercent\n    vramGpu {\n      index\n      name\n      vendor\n      hasVram\n    }\n  }\n": types.CardReadingsFragmentDoc,
    "\n  query Device($id: ID!) {\n    device(id: $id) {\n      ...DeviceDetailFields\n    }\n  }\n": types.DeviceDocument,
    "\n  query Sites {\n    sites {\n      id\n      name\n      subnet\n    }\n  }\n": types.SitesDocument,
    "\n  mutation CreateSite($name: String!, $subnet: Cidr!) {\n    createSite(name: $name, subnet: $subnet) {\n      id\n      name\n      subnet\n    }\n  }\n": types.CreateSiteDocument,
    "\n  mutation CreateDevice($input: CreateDeviceInput!) {\n    createDevice(input: $input) {\n      ...DeviceDetailFields\n    }\n  }\n": types.CreateDeviceDocument,
    "\n  mutation UpdateDevice($id: ID!, $input: UpdateDeviceInput!) {\n    updateDevice(id: $id, input: $input) {\n      ...DeviceDetailFields\n    }\n  }\n": types.UpdateDeviceDocument,
    "\n  mutation DeleteDevice($id: ID!) {\n    deleteDevice(id: $id)\n  }\n": types.DeleteDeviceDocument,
    "\n  mutation SetSshCredential($deviceId: ID!, $credential: SshCredentialInput!) {\n    setSshCredential(deviceId: $deviceId, credential: $credential) {\n      credentialStored\n    }\n  }\n": types.SetSshCredentialDocument,
    "\n  mutation ClearSshCredential($deviceId: ID!) {\n    clearSshCredential(deviceId: $deviceId) {\n      credentialStored\n    }\n  }\n": types.ClearSshCredentialDocument,
    "\n  mutation ClearRdpCredential($deviceId: ID!) {\n    clearRdpCredential(deviceId: $deviceId) {\n      credentialStored\n    }\n  }\n": types.ClearRdpCredentialDocument,
    "\n  mutation ClearVncCredential($deviceId: ID!) {\n    clearVncCredential(deviceId: $deviceId) {\n      credentialStored\n    }\n  }\n": types.ClearVncCredentialDocument,
    "\n  query DevicePolicy($id: ID!) {\n    device(id: $id) {\n      sessionLimit\n    }\n  }\n": types.DevicePolicyDocument,
    "\n  mutation SetDevicePolicy($id: ID!, $input: UpdateDeviceInput!) {\n    updateDevice(id: $id, input: $input) {\n      sessionLimit\n    }\n  }\n": types.SetDevicePolicyDocument,
    "\n  query Events($filter: EventFilter, $first: Int, $after: String) {\n    events(filter: $filter, first: $first, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          at\n          severity\n          source\n          deviceId\n          deviceName\n          message\n        }\n      }\n    }\n  }\n": types.EventsDocument,
    "\n  subscription EventAdded($minSeverity: Severity) {\n    eventAdded(minSeverity: $minSeverity) {\n      id\n      at\n      severity\n      source\n      deviceId\n      deviceName\n      message\n    }\n  }\n": types.EventAddedDocument,
    "\n  query DeviceMetrics($id: ID!, $range: MetricRange!) {\n    device(id: $id) {\n      id\n      metricsSource\n      gpus {\n        index\n        name\n        vendor\n        hasVram\n      }\n      readings {\n        at\n        cpuPercent\n        memoryPercent\n        diskPercent\n        diskMount\n        otherDrives\n        vramPercent\n        vramGpu {\n          index\n          name\n          vendor\n          hasVram\n        }\n      }\n      storage {\n        drives {\n          name\n          kind\n          sizeBytes\n          mounted\n          mounts {\n            path\n            usedBytes\n            sizeBytes\n            percent\n          }\n          readBps\n          writeBps\n          busyPercent\n        }\n      }\n      metrics(range: $range) {\n        kind\n        label\n        gpuIndex\n        points {\n          at\n          value\n          min\n          max\n        }\n      }\n    }\n  }\n": types.DeviceMetricsDocument,
    "\n  subscription MetricsAdded($deviceId: ID!) {\n    metricsAdded(deviceId: $deviceId) {\n      deviceId\n      at\n      samples {\n        kind\n        label\n        value\n      }\n      readings {\n        at\n        cpuPercent\n        memoryPercent\n        diskPercent\n        diskMount\n        otherDrives\n        vramPercent\n        vramGpu {\n          index\n          name\n          vendor\n          hasVram\n        }\n      }\n    }\n  }\n": types.MetricsAddedDocument,
    "\n  query AgentStatus($id: ID!) {\n    device(id: $id) {\n      id\n      metricsSource\n      metricsStale\n    }\n  }\n": types.AgentStatusDocument,
    "\n  mutation EnrolAgent($deviceId: ID!) {\n    enrolAgent(deviceId: $deviceId) {\n      code\n      expiresAt\n    }\n  }\n": types.EnrolAgentDocument,
    "\n  mutation RevokeAgentToken($deviceId: ID!) {\n    revokeAgentToken(deviceId: $deviceId) {\n      id\n      metricsSource\n      metricsStale\n    }\n  }\n": types.RevokeAgentTokenDocument,
    "\n  fragment DeliveryFields on Delivery {\n    id\n    channelId\n    kind\n    at\n    ok\n    reason\n    code\n  }\n": types.DeliveryFieldsFragmentDoc,
    "\n  fragment NotificationChannelFields on NotificationChannel {\n    id\n    name\n    kind\n    onByDefault\n    settings {\n      __typename\n      ... on WebhookSettings {\n        origin\n        method\n        headerNames\n      }\n      ... on NtfySettings {\n        server\n        priority\n        tokenStored\n      }\n      ... on EmailSettings {\n        host\n        port\n        security\n        username\n        from\n        to\n        passwordStored\n      }\n      ... on AppriseSettings {\n        server\n        mode\n        tags\n      }\n    }\n    template {\n      title\n      body\n    }\n    lastDelivery {\n      ...DeliveryFields\n    }\n  }\n": types.NotificationChannelFieldsFragmentDoc,
    "\n  fragment NotificationSettingsFields on NotificationSettings {\n    certificateDays\n    template {\n      title\n      body\n    }\n    placeholders\n  }\n": types.NotificationSettingsFieldsFragmentDoc,
    "\n  query NotificationChannels {\n    notificationChannels {\n      ...NotificationChannelFields\n    }\n  }\n": types.NotificationChannelsDocument,
    "\n  query NotificationSettings {\n    notificationSettings {\n      ...NotificationSettingsFields\n    }\n  }\n": types.NotificationSettingsDocument,
    "\n  mutation CreateNotificationChannel($input: NotificationChannelInput!) {\n    createNotificationChannel(input: $input) {\n      ...NotificationChannelFields\n    }\n  }\n": types.CreateNotificationChannelDocument,
    "\n  mutation UpdateNotificationChannel($id: ID!, $input: NotificationChannelInput!) {\n    updateNotificationChannel(id: $id, input: $input) {\n      ...NotificationChannelFields\n    }\n  }\n": types.UpdateNotificationChannelDocument,
    "\n  mutation DeleteNotificationChannel($id: ID!) {\n    deleteNotificationChannel(id: $id)\n  }\n": types.DeleteNotificationChannelDocument,
    "\n  mutation SendTestNotification($id: ID!) {\n    sendTestNotification(id: $id) {\n      ...DeliveryFields\n    }\n  }\n": types.SendTestNotificationDocument,
    "\n  mutation SetNotificationSettings($input: NotificationSettingsInput!) {\n    setNotificationSettings(input: $input) {\n      ...NotificationSettingsFields\n    }\n  }\n": types.SetNotificationSettingsDocument,
    "\n  query Retention {\n    retention {\n      subject\n      days\n      minDays\n      maxDays\n      defaultDays\n    }\n  }\n": types.RetentionDocument,
    "\n  mutation SetRetention($subject: RetentionSubject!, $days: Int!) {\n    setRetention(subject: $subject, days: $days) {\n      subject\n      days\n      minDays\n      maxDays\n      defaultDays\n    }\n  }\n": types.SetRetentionDocument,
    "\n  mutation StartSession($input: StartSessionInput!) {\n    startSession(input: $input) {\n      sessionId\n      token\n      tunnelPath\n      expiresAt\n      wakeSent\n    }\n  }\n": types.StartSessionDocument,
    "\n  mutation EndSession($id: ID!) {\n    endSession(id: $id) {\n      id\n      deviceId\n      deviceName\n      protocol\n      startedAt\n      endedAt\n      endReason\n    }\n  }\n": types.EndSessionDocument,
    "\n  mutation TestConnection($input: ConnectionTestInput!) {\n    testConnection(input: $input) {\n      reachable\n      authenticated\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n      hostKeyChanged\n      message\n    }\n  }\n": types.TestConnectionDocument,
    "\n  mutation AcceptHostKey($deviceId: ID!, $fingerprintSha256: String!) {\n    acceptHostKey(deviceId: $deviceId, fingerprintSha256: $fingerprintSha256) {\n      algorithm\n      fingerprintSha256\n      firstSeenAt\n    }\n  }\n": types.AcceptHostKeyDocument,
    "\n  mutation TestRemoteConnection($input: ConnectionTestInput!) {\n    testConnection(input: $input) {\n      reachable\n      authenticated\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n      hostKeyChanged\n      message\n      serverCertificate {\n        fingerprintSha256\n        subject\n        notAfter\n        firstSeenAt\n      }\n      certificateChanged\n      certificateMatched\n    }\n  }\n": types.TestRemoteConnectionDocument,
    "\n  mutation AcceptServerCertificate($deviceId: ID!, $fingerprintSha256: String!) {\n    acceptServerCertificate(deviceId: $deviceId, fingerprintSha256: $fingerprintSha256) {\n      fingerprintSha256\n      subject\n      notAfter\n      firstSeenAt\n    }\n  }\n": types.AcceptServerCertificateDocument,
    "\n  query DeviceTransfer($id: ID!) {\n    device(id: $id) {\n      transfer {\n        copy\n        paste\n      }\n    }\n  }\n": types.DeviceTransferDocument,
    "\n  mutation SetRdpCredential($deviceId: ID!, $credential: PasswordCredentialInput!) {\n    setRdpCredential(deviceId: $deviceId, credential: $credential) {\n      credentialStored\n    }\n  }\n": types.SetRdpCredentialDocument,
    "\n  mutation SetVncCredential($deviceId: ID!, $credential: PasswordCredentialInput!) {\n    setVncCredential(deviceId: $deviceId, credential: $credential) {\n      credentialStored\n    }\n  }\n": types.SetVncCredentialDocument,
    "\n  query DeviceSessions($deviceId: ID!, $first: Int, $after: String) {\n    sessions(deviceId: $deviceId, first: $first, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          protocol\n          operatorName\n          startedAt\n          endedAt\n          endReason\n        }\n      }\n    }\n  }\n": types.DeviceSessionsDocument,
    "\n  mutation OpenFiles($sessionId: ID!) {\n    openFiles(sessionId: $sessionId) {\n      sessionId\n      token\n      tunnelPath\n      expiresAt\n    }\n  }\n": types.OpenFilesDocument,
    "\n  query LiveSessions($first: Int) {\n    sessions(live: true, first: $first) {\n      edges {\n        node {\n          id\n          deviceId\n          deviceName\n          protocol\n          operatorName\n          startedAt\n          clientAddr\n        }\n      }\n    }\n  }\n": types.LiveSessionsDocument,
    "\n  subscription LiveSessionChanged {\n    sessionChanged {\n      session {\n        id\n        deviceId\n        deviceName\n        protocol\n        operatorName\n        startedAt\n        clientAddr\n        endedAt\n        endReason\n      }\n    }\n  }\n": types.LiveSessionChangedDocument,
    "\n  mutation WakeDevice($deviceId: ID!) {\n    wakeDevice(deviceId: $deviceId) {\n      sentAt\n      relayName\n    }\n  }\n": types.WakeDeviceDocument,
    "\n  fragment VpnNodeFields on VpnNode {\n    name\n    address\n    os\n    role\n    tags\n    routes\n    waitingRoutes\n    online\n    lastSeen\n    keyExpiry\n    keyState\n    path\n    relayRegion\n    inAnima\n    device {\n      id\n      name\n    }\n  }\n": types.VpnNodeFieldsFragmentDoc,
    "\n  fragment VpnPictureFields on VpnPicture {\n    readAt\n    fresh\n    hub {\n      ...VpnNodeFields\n    }\n    sites {\n      site {\n        id\n        name\n        subnet\n      }\n      router {\n        ...VpnNodeFields\n      }\n      state\n      path\n      relayRegion\n      hasPingCheck\n      waitingRouter {\n        ...VpnNodeFields\n      }\n    }\n    nodes {\n      ...VpnNodeFields\n    }\n    broker\n  }\n": types.VpnPictureFieldsFragmentDoc,
    "\n  query Vpn {\n    vpn {\n      ...VpnPictureFields\n    }\n  }\n": types.VpnDocument,
    "\n  subscription VpnChanged {\n    vpnChanged {\n      ...VpnPictureFields\n    }\n  }\n": types.VpnChangedDocument,
    "\n  mutation CreateVpnInvite {\n    createVpnInvite {\n      key\n      expiresAt\n      steps\n    }\n  }\n": types.CreateVpnInviteDocument,
    "\n  mutation CreateRouterKey($siteId: ID!) {\n    createRouterKey(siteId: $siteId) {\n      key\n      expiresAt\n      steps\n    }\n  }\n": types.CreateRouterKeyDocument,
    "\n  mutation RegisterVpnNode($authId: String!) {\n    registerVpnNode(authId: $authId) {\n      name\n      address\n      keyExpiry\n    }\n  }\n": types.RegisterVpnNodeDocument,
    "\n  mutation RenewVpnKey($name: String!) {\n    renewVpnKey(name: $name) {\n      name\n      address\n      keyExpiry\n    }\n  }\n": types.RenewVpnKeyDocument,
    "\n  mutation ApproveSiteRoute($siteId: ID!) {\n    approveSiteRoute(siteId: $siteId)\n  }\n": types.ApproveSiteRouteDocument,
    "\n  fragment WebInterfaceSuggestionFields on WebInterfaceSuggestion {\n    checkId\n    url\n    label\n  }\n": types.WebInterfaceSuggestionFieldsFragmentDoc,
    "\n  fragment WebInterfaceFields on WebInterface {\n    id\n    deviceId\n    label\n    url\n    openIn\n    pinnedFingerprintSha256\n    pinnedAt\n  }\n": types.WebInterfaceFieldsFragmentDoc,
    "\n  mutation CreateWebInterface($deviceId: ID!, $input: WebInterfaceInput!) {\n    createWebInterface(deviceId: $deviceId, input: $input) {\n      ...WebInterfaceFields\n    }\n  }\n": types.CreateWebInterfaceDocument,
    "\n  mutation UpdateWebInterface($id: ID!, $input: WebInterfaceInput!) {\n    updateWebInterface(id: $id, input: $input) {\n      ...WebInterfaceFields\n    }\n  }\n": types.UpdateWebInterfaceDocument,
    "\n  mutation DeleteWebInterface($id: ID!) {\n    deleteWebInterface(id: $id)\n  }\n": types.DeleteWebInterfaceDocument,
    "\n  mutation AcceptWebInterfaceSuggestion($checkId: ID!) {\n    acceptWebInterfaceSuggestion(checkId: $checkId) {\n      ...WebInterfaceFields\n    }\n  }\n": types.AcceptWebInterfaceSuggestionDocument,
    "\n  mutation DismissWebInterfaceSuggestion($checkId: ID!) {\n    dismissWebInterfaceSuggestion(checkId: $checkId)\n  }\n": types.DismissWebInterfaceSuggestionDocument,
    "\n  mutation AcceptWebCertificate($id: ID!, $fingerprintSha256: String!) {\n    acceptWebCertificate(id: $id, fingerprintSha256: $fingerprintSha256) {\n      id\n      pinnedFingerprintSha256\n      pinnedAt\n    }\n  }\n": types.AcceptWebCertificateDocument,
};

/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 *
 *
 * @example
 * ```ts
 * const query = graphql(`query GetUser($id: ID!) { user(id: $id) { name } }`);
 * ```
 *
 * The query argument is unknown!
 * Please regenerate the types.
 */
export function graphql(source: string): unknown;

/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation Login($input: LoginInput!) {\n    login(input: $input) {\n      token\n    }\n  }\n"): (typeof documents)["\n  mutation Login($input: LoginInput!) {\n    login(input: $input) {\n      token\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation SignInBrowser($input: LoginInput!) {\n    signInBrowser(input: $input) {\n      expiresAt\n    }\n  }\n"): (typeof documents)["\n  mutation SignInBrowser($input: LoginInput!) {\n    signInBrowser(input: $input) {\n      expiresAt\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation Logout {\n    logout\n  }\n"): (typeof documents)["\n  mutation Logout {\n    logout\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Me {\n    me {\n      username\n    }\n  }\n"): (typeof documents)["\n  query Me {\n    me {\n      username\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query ApiKeys {\n    apiKeys {\n      id\n      name\n      scope\n      prefix\n      expiresAt\n      createdAt\n      lastUsedAt\n    }\n  }\n"): (typeof documents)["\n  query ApiKeys {\n    apiKeys {\n      id\n      name\n      scope\n      prefix\n      expiresAt\n      createdAt\n      lastUsedAt\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CreateApiKey($input: ApiKeyInput!) {\n    createApiKey(input: $input) {\n      key {\n        id\n        name\n        scope\n        prefix\n        expiresAt\n        createdAt\n        lastUsedAt\n      }\n      secret\n    }\n  }\n"): (typeof documents)["\n  mutation CreateApiKey($input: ApiKeyInput!) {\n    createApiKey(input: $input) {\n      key {\n        id\n        name\n        scope\n        prefix\n        expiresAt\n        createdAt\n        lastUsedAt\n      }\n      secret\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation RevokeApiKey($id: ID!) {\n    revokeApiKey(id: $id)\n  }\n"): (typeof documents)["\n  mutation RevokeApiKey($id: ID!) {\n    revokeApiKey(id: $id)\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Builders {\n    builders {\n      id\n      name\n      platform {\n        os\n        arch\n      }\n      online\n    }\n  }\n"): (typeof documents)["\n  query Builders {\n    builders {\n      id\n      name\n      platform {\n        os\n        arch\n      }\n      online\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation EnrolBuilder($name: String!, $platform: BuilderPlatformInput!) {\n    enrolBuilder(name: $name, platform: $platform) {\n      builder {\n        id\n      }\n      token\n    }\n  }\n"): (typeof documents)["\n  mutation EnrolBuilder($name: String!, $platform: BuilderPlatformInput!) {\n    enrolBuilder(name: $name, platform: $platform) {\n      builder {\n        id\n      }\n      token\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation RevokeBuilder($id: ID!) {\n    revokeBuilder(id: $id)\n  }\n"): (typeof documents)["\n  mutation RevokeBuilder($id: ID!) {\n    revokeBuilder(id: $id)\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation ClaimBuildJob($platform: BuilderPlatformInput!) {\n    claimBuildJob(platform: $platform) {\n      id\n      target\n      version\n      commit\n    }\n  }\n"): (typeof documents)["\n  mutation ClaimBuildJob($platform: BuilderPlatformInput!) {\n    claimBuildJob(platform: $platform) {\n      id\n      target\n      version\n      commit\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation ReportBuildJob($id: ID!, $state: BuildJobState, $logLines: [String!]!, $failureReason: String) {\n    reportBuildJob(id: $id, state: $state, logLines: $logLines, failureReason: $failureReason) {\n      id\n      state\n    }\n  }\n"): (typeof documents)["\n  mutation ReportBuildJob($id: ID!, $state: BuildJobState, $logLines: [String!]!, $failureReason: String) {\n    reportBuildJob(id: $id, state: $state, logLines: $logLines, failureReason: $failureReason) {\n      id\n      state\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation BuildUploadUrls($id: ID!, $files: [BuildFileInput!]!) {\n    buildUploadUrls(id: $id, files: $files) {\n      name\n      url\n      headers {\n        name\n        value\n      }\n      expiresAt\n    }\n  }\n"): (typeof documents)["\n  mutation BuildUploadUrls($id: ID!, $files: [BuildFileInput!]!) {\n    buildUploadUrls(id: $id, files: $files) {\n      name\n      url\n      headers {\n        name\n        value\n      }\n      expiresAt\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation ReportBuildUploaded($id: ID!, $files: [BuildFileInput!]!, $feed: String!) {\n    reportBuildUploaded(id: $id, files: $files, feed: $feed) {\n      id\n      state\n    }\n  }\n"): (typeof documents)["\n  mutation ReportBuildUploaded($id: ID!, $files: [BuildFileInput!]!, $feed: String!) {\n    reportBuildUploaded(id: $id, files: $files, feed: $feed) {\n      id\n      state\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Releases($first: Int) {\n    releases(first: $first) {\n      edges {\n        node {\n          id\n          version\n          commit\n          createdAt\n          createdBy\n          jobs {\n            id\n            releaseId\n            target\n            state\n            failureReason\n            attempts\n            builderName\n            stateChangedAt\n          }\n        }\n      }\n    }\n  }\n"): (typeof documents)["\n  query Releases($first: Int) {\n    releases(first: $first) {\n      edges {\n        node {\n          id\n          version\n          commit\n          createdAt\n          createdBy\n          jobs {\n            id\n            releaseId\n            target\n            state\n            failureReason\n            attempts\n            builderName\n            stateChangedAt\n          }\n        }\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query NextReleaseVersion {\n    nextReleaseVersion\n  }\n"): (typeof documents)["\n  query NextReleaseVersion {\n    nextReleaseVersion\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CreateRelease($version: String!, $commit: String!, $targets: [BuildTarget!]!) {\n    createRelease(version: $version, commit: $commit, targets: $targets) {\n      id\n    }\n  }\n"): (typeof documents)["\n  mutation CreateRelease($version: String!, $commit: String!, $targets: [BuildTarget!]!) {\n    createRelease(version: $version, commit: $commit, targets: $targets) {\n      id\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CancelBuildJob($id: ID!) {\n    cancelBuildJob(id: $id) {\n      id\n      state\n    }\n  }\n"): (typeof documents)["\n  mutation CancelBuildJob($id: ID!) {\n    cancelBuildJob(id: $id) {\n      id\n      state\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query BuildJobLog($id: ID!, $after: Int, $first: Int) {\n    buildJobLog(id: $id, after: $after, first: $first) {\n      sequence\n      text\n    }\n  }\n"): (typeof documents)["\n  query BuildJobLog($id: ID!, $after: Int, $first: Int) {\n    buildJobLog(id: $id, after: $after, first: $first) {\n      sequence\n      text\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  subscription BuildJobUpdated($jobId: ID) {\n    buildJobUpdated(jobId: $jobId) {\n      job {\n        id\n        releaseId\n        target\n        state\n        failureReason\n        attempts\n        builderName\n        stateChangedAt\n      }\n      logLines {\n        sequence\n        text\n      }\n    }\n  }\n"): (typeof documents)["\n  subscription BuildJobUpdated($jobId: ID) {\n    buildJobUpdated(jobId: $jobId) {\n      job {\n        id\n        releaseId\n        target\n        state\n        failureReason\n        attempts\n        builderName\n        stateChangedAt\n      }\n      logLines {\n        sequence\n        text\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  subscription HeartbeatAdded {\n    heartbeatAdded(deviceId: null) {\n      checkId\n      deviceId\n      heartbeat {\n        at\n        status\n        responseTimeMs\n      }\n    }\n  }\n"): (typeof documents)["\n  subscription HeartbeatAdded {\n    heartbeatAdded(deviceId: null) {\n      checkId\n      deviceId\n      heartbeat {\n        at\n        status\n        responseTimeMs\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment CheckFields on Check {\n    id\n    parent {\n      id\n    }\n    type\n    name\n    intervalSeconds\n    retries\n    retryIntervalSeconds\n    degradedThresholdMs\n    invert\n    paused\n    status\n    certificateDaysLeft\n    domainDaysLeft\n    lastHeartbeat {\n      at\n      status\n      responseTimeMs\n      message\n    }\n    config {\n      __typename\n      ... on IcmpConfig {\n        timeoutMs\n        ipVersion\n      }\n      ... on TcpConfig {\n        port\n        timeoutMs\n      }\n      ... on TlsConfig {\n        port\n        serverName\n        timeoutMs\n      }\n      ... on HttpConfig {\n        url\n        method\n        headers {\n          name\n          value\n        }\n        body\n        maxRedirects\n        acceptedStatus {\n          from\n          to\n        }\n        keyword\n        jsonQuery\n        jsonExpected\n        authKind\n        verifyTls\n        timeoutMs\n      }\n      ... on DnsConfig {\n        name\n        recordType\n        resolver\n        expected\n      }\n      ... on PushConfig {\n        pushPath\n      }\n      ... on GroupConfig {\n        children {\n          id\n          name\n        }\n      }\n      ... on DomainExpiryConfig {\n        domain\n        warnDays\n        timeoutMs\n      }\n    }\n  }\n"): (typeof documents)["\n  fragment CheckFields on Check {\n    id\n    parent {\n      id\n    }\n    type\n    name\n    intervalSeconds\n    retries\n    retryIntervalSeconds\n    degradedThresholdMs\n    invert\n    paused\n    status\n    certificateDaysLeft\n    domainDaysLeft\n    lastHeartbeat {\n      at\n      status\n      responseTimeMs\n      message\n    }\n    config {\n      __typename\n      ... on IcmpConfig {\n        timeoutMs\n        ipVersion\n      }\n      ... on TcpConfig {\n        port\n        timeoutMs\n      }\n      ... on TlsConfig {\n        port\n        serverName\n        timeoutMs\n      }\n      ... on HttpConfig {\n        url\n        method\n        headers {\n          name\n          value\n        }\n        body\n        maxRedirects\n        acceptedStatus {\n          from\n          to\n        }\n        keyword\n        jsonQuery\n        jsonExpected\n        authKind\n        verifyTls\n        timeoutMs\n      }\n      ... on DnsConfig {\n        name\n        recordType\n        resolver\n        expected\n      }\n      ... on PushConfig {\n        pushPath\n      }\n      ... on GroupConfig {\n        children {\n          id\n          name\n        }\n      }\n      ... on DomainExpiryConfig {\n        domain\n        warnDays\n        timeoutMs\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment DockerCheckConfig on DockerConfig {\n    __typename\n    container\n    timeoutMs\n  }\n"): (typeof documents)["\n  fragment DockerCheckConfig on DockerConfig {\n    __typename\n    container\n    timeoutMs\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment SystemdCheckConfig on SystemdConfig {\n    __typename\n    unit\n    timeoutMs\n  }\n"): (typeof documents)["\n  fragment SystemdCheckConfig on SystemdConfig {\n    __typename\n    unit\n    timeoutMs\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment SnmpCheckConfig on SnmpConfig {\n    __typename\n    port\n    version\n    oid\n    expected\n    username\n    timeoutMs\n  }\n"): (typeof documents)["\n  fragment SnmpCheckConfig on SnmpConfig {\n    __typename\n    port\n    version\n    oid\n    expected\n    username\n    timeoutMs\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment TailscalePingCheckConfig on TailscalePingConfig {\n    __typename\n    target\n    timeoutMs\n  }\n"): (typeof documents)["\n  fragment TailscalePingCheckConfig on TailscalePingConfig {\n    __typename\n    target\n    timeoutMs\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query CheckDefaults {\n    checkDefaults {\n      intervalSeconds\n      retries\n      retryIntervalSeconds\n      degradedThresholdMs\n    }\n  }\n"): (typeof documents)["\n  query CheckDefaults {\n    checkDefaults {\n      intervalSeconds\n      retries\n      retryIntervalSeconds\n      degradedThresholdMs\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation SetCheckDefaults($input: CheckDefaultsInput!) {\n    setCheckDefaults(input: $input) {\n      intervalSeconds\n      retries\n      retryIntervalSeconds\n      degradedThresholdMs\n    }\n  }\n"): (typeof documents)["\n  mutation SetCheckDefaults($input: CheckDefaultsInput!) {\n    setCheckDefaults(input: $input) {\n      intervalSeconds\n      retries\n      retryIntervalSeconds\n      degradedThresholdMs\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query DeviceChecks($deviceId: ID!) {\n    device(id: $deviceId) {\n      id\n      checks {\n        ...CheckFields\n      }\n    }\n  }\n"): (typeof documents)["\n  query DeviceChecks($deviceId: ID!) {\n    device(id: $deviceId) {\n      id\n      checks {\n        ...CheckFields\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query CheckHeartbeats($id: ID!, $first: Int, $after: String) {\n    check(id: $id) {\n      id\n      heartbeats(first: $first, after: $after) {\n        pageInfo {\n          hasNextPage\n          endCursor\n        }\n        edges {\n          node {\n            at\n            status\n            responseTimeMs\n            message\n          }\n        }\n      }\n    }\n  }\n"): (typeof documents)["\n  query CheckHeartbeats($id: ID!, $first: Int, $after: String) {\n    check(id: $id) {\n      id\n      heartbeats(first: $first, after: $after) {\n        pageInfo {\n          hasNextPage\n          endCursor\n        }\n        edges {\n          node {\n            at\n            status\n            responseTimeMs\n            message\n          }\n        }\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CreateCheck($input: CreateCheckInput!) {\n    createCheck(input: $input) {\n      ...CheckFields\n    }\n  }\n"): (typeof documents)["\n  mutation CreateCheck($input: CreateCheckInput!) {\n    createCheck(input: $input) {\n      ...CheckFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UpdateCheck($id: ID!, $input: UpdateCheckInput!) {\n    updateCheck(id: $id, input: $input) {\n      ...CheckFields\n    }\n  }\n"): (typeof documents)["\n  mutation UpdateCheck($id: ID!, $input: UpdateCheckInput!) {\n    updateCheck(id: $id, input: $input) {\n      ...CheckFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation DeleteCheck($id: ID!) {\n    deleteCheck(id: $id)\n  }\n"): (typeof documents)["\n  mutation DeleteCheck($id: ID!) {\n    deleteCheck(id: $id)\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation SetCheckPaused($id: ID!, $paused: Boolean!) {\n    setCheckPaused(id: $id, paused: $paused) {\n      ...CheckFields\n    }\n  }\n"): (typeof documents)["\n  mutation SetCheckPaused($id: ID!, $paused: Boolean!) {\n    setCheckPaused(id: $id, paused: $paused) {\n      ...CheckFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query CheckNotifications($id: ID!) {\n    check(id: $id) {\n      notifications {\n        channels {\n          id\n        }\n        resendEveryBeats\n      }\n    }\n  }\n"): (typeof documents)["\n  query CheckNotifications($id: ID!) {\n    check(id: $id) {\n      notifications {\n        channels {\n          id\n        }\n        resendEveryBeats\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation SetCheckNotifications($checkId: ID!, $input: CheckNotificationsInput!) {\n    setCheckNotifications(checkId: $checkId, input: $input) {\n      channels {\n        id\n      }\n      resendEveryBeats\n    }\n  }\n"): (typeof documents)["\n  mutation SetCheckNotifications($checkId: ID!, $input: CheckNotificationsInput!) {\n    setCheckNotifications(checkId: $checkId, input: $input) {\n      channels {\n        id\n      }\n      resendEveryBeats\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Devices($filter: DeviceFilter, $sort: DeviceSort, $first: Int, $after: String) {\n    devices(filter: $filter, sort: $sort, first: $first, after: $after) {\n      totalCount\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          name\n          host\n          ip\n          tags\n          maintenance\n          status\n          uptime(window: DAY)\n          os\n          distribution\n          macAddress\n          metricsSource\n          readings {\n            ...CardReadings\n          }\n          accessWarnings\n          webInterfaces {\n            ...WebInterfaceFields\n          }\n          rdp {\n            enabled\n          }\n          vnc {\n            enabled\n            username\n          }\n          ssh {\n            enabled\n            hostKey {\n              fingerprintSha256\n            }\n          }\n          checks {\n            id\n            heartbeats(first: 50) {\n              edges {\n                node {\n                  at\n                  status\n                  responseTimeMs\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  }\n"): (typeof documents)["\n  query Devices($filter: DeviceFilter, $sort: DeviceSort, $first: Int, $after: String) {\n    devices(filter: $filter, sort: $sort, first: $first, after: $after) {\n      totalCount\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          name\n          host\n          ip\n          tags\n          maintenance\n          status\n          uptime(window: DAY)\n          os\n          distribution\n          macAddress\n          metricsSource\n          readings {\n            ...CardReadings\n          }\n          accessWarnings\n          webInterfaces {\n            ...WebInterfaceFields\n          }\n          rdp {\n            enabled\n          }\n          vnc {\n            enabled\n            username\n          }\n          ssh {\n            enabled\n            hostKey {\n              fingerprintSha256\n            }\n          }\n          checks {\n            id\n            heartbeats(first: 50) {\n              edges {\n                node {\n                  at\n                  status\n                  responseTimeMs\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query DeviceHosts($after: String) {\n    devices(first: 200, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          host\n        }\n      }\n    }\n  }\n"): (typeof documents)["\n  query DeviceHosts($after: String) {\n    devices(first: 200, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          host\n        }\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Tags {\n    tags\n  }\n"): (typeof documents)["\n  query Tags {\n    tags\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  subscription DeviceStatusChanged {\n    deviceStatusChanged(deviceId: null) {\n      deviceId\n      status\n      at\n    }\n  }\n"): (typeof documents)["\n  subscription DeviceStatusChanged {\n    deviceStatusChanged(deviceId: null) {\n      deviceId\n      status\n      at\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  subscription CardReadingsAdded {\n    metricsAdded(deviceId: null) {\n      deviceId\n      readings {\n        ...CardReadings\n      }\n    }\n  }\n"): (typeof documents)["\n  subscription CardReadingsAdded {\n    metricsAdded(deviceId: null) {\n      deviceId\n      readings {\n        ...CardReadings\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment DeviceDetailFields on Device {\n    id\n    name\n    host\n    ip\n    kind\n    os\n    distribution\n    tags\n    maintenance\n    status\n    site {\n      id\n      name\n    }\n    ssh {\n      enabled\n      port\n      username\n      auth\n      credentialStored\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n    }\n    macAddress\n    metricsSource\n    accessWarnings\n    rdp {\n      enabled\n      port\n      username\n      domain\n      security\n      ignoreCertificate\n      resizeMethod\n      audioOut\n      microphone\n      sharedDrive\n      credentialStored\n      serverCertificate {\n        fingerprintSha256\n        subject\n        notAfter\n        firstSeenAt\n      }\n    }\n    vnc {\n      enabled\n      port\n      username\n      credentialStored\n    }\n    transfer {\n      upload\n      download\n      copy\n      paste\n    }\n    webInterfaces {\n      ...WebInterfaceFields\n    }\n    webInterfaceSuggestions {\n      ...WebInterfaceSuggestionFields\n    }\n    day: uptime(window: DAY)\n    week: uptime(window: WEEK)\n    month: uptime(window: MONTH)\n  }\n"): (typeof documents)["\n  fragment DeviceDetailFields on Device {\n    id\n    name\n    host\n    ip\n    kind\n    os\n    distribution\n    tags\n    maintenance\n    status\n    site {\n      id\n      name\n    }\n    ssh {\n      enabled\n      port\n      username\n      auth\n      credentialStored\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n    }\n    macAddress\n    metricsSource\n    accessWarnings\n    rdp {\n      enabled\n      port\n      username\n      domain\n      security\n      ignoreCertificate\n      resizeMethod\n      audioOut\n      microphone\n      sharedDrive\n      credentialStored\n      serverCertificate {\n        fingerprintSha256\n        subject\n        notAfter\n        firstSeenAt\n      }\n    }\n    vnc {\n      enabled\n      port\n      username\n      credentialStored\n    }\n    transfer {\n      upload\n      download\n      copy\n      paste\n    }\n    webInterfaces {\n      ...WebInterfaceFields\n    }\n    webInterfaceSuggestions {\n      ...WebInterfaceSuggestionFields\n    }\n    day: uptime(window: DAY)\n    week: uptime(window: WEEK)\n    month: uptime(window: MONTH)\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment CardReadings on Readings {\n    at\n    cpuPercent\n    memoryPercent\n    diskPercent\n    diskMount\n    otherDrives\n    vramPercent\n    vramGpu {\n      index\n      name\n      vendor\n      hasVram\n    }\n  }\n"): (typeof documents)["\n  fragment CardReadings on Readings {\n    at\n    cpuPercent\n    memoryPercent\n    diskPercent\n    diskMount\n    otherDrives\n    vramPercent\n    vramGpu {\n      index\n      name\n      vendor\n      hasVram\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Device($id: ID!) {\n    device(id: $id) {\n      ...DeviceDetailFields\n    }\n  }\n"): (typeof documents)["\n  query Device($id: ID!) {\n    device(id: $id) {\n      ...DeviceDetailFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Sites {\n    sites {\n      id\n      name\n      subnet\n    }\n  }\n"): (typeof documents)["\n  query Sites {\n    sites {\n      id\n      name\n      subnet\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CreateSite($name: String!, $subnet: Cidr!) {\n    createSite(name: $name, subnet: $subnet) {\n      id\n      name\n      subnet\n    }\n  }\n"): (typeof documents)["\n  mutation CreateSite($name: String!, $subnet: Cidr!) {\n    createSite(name: $name, subnet: $subnet) {\n      id\n      name\n      subnet\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CreateDevice($input: CreateDeviceInput!) {\n    createDevice(input: $input) {\n      ...DeviceDetailFields\n    }\n  }\n"): (typeof documents)["\n  mutation CreateDevice($input: CreateDeviceInput!) {\n    createDevice(input: $input) {\n      ...DeviceDetailFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UpdateDevice($id: ID!, $input: UpdateDeviceInput!) {\n    updateDevice(id: $id, input: $input) {\n      ...DeviceDetailFields\n    }\n  }\n"): (typeof documents)["\n  mutation UpdateDevice($id: ID!, $input: UpdateDeviceInput!) {\n    updateDevice(id: $id, input: $input) {\n      ...DeviceDetailFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation DeleteDevice($id: ID!) {\n    deleteDevice(id: $id)\n  }\n"): (typeof documents)["\n  mutation DeleteDevice($id: ID!) {\n    deleteDevice(id: $id)\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation SetSshCredential($deviceId: ID!, $credential: SshCredentialInput!) {\n    setSshCredential(deviceId: $deviceId, credential: $credential) {\n      credentialStored\n    }\n  }\n"): (typeof documents)["\n  mutation SetSshCredential($deviceId: ID!, $credential: SshCredentialInput!) {\n    setSshCredential(deviceId: $deviceId, credential: $credential) {\n      credentialStored\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation ClearSshCredential($deviceId: ID!) {\n    clearSshCredential(deviceId: $deviceId) {\n      credentialStored\n    }\n  }\n"): (typeof documents)["\n  mutation ClearSshCredential($deviceId: ID!) {\n    clearSshCredential(deviceId: $deviceId) {\n      credentialStored\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation ClearRdpCredential($deviceId: ID!) {\n    clearRdpCredential(deviceId: $deviceId) {\n      credentialStored\n    }\n  }\n"): (typeof documents)["\n  mutation ClearRdpCredential($deviceId: ID!) {\n    clearRdpCredential(deviceId: $deviceId) {\n      credentialStored\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation ClearVncCredential($deviceId: ID!) {\n    clearVncCredential(deviceId: $deviceId) {\n      credentialStored\n    }\n  }\n"): (typeof documents)["\n  mutation ClearVncCredential($deviceId: ID!) {\n    clearVncCredential(deviceId: $deviceId) {\n      credentialStored\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query DevicePolicy($id: ID!) {\n    device(id: $id) {\n      sessionLimit\n    }\n  }\n"): (typeof documents)["\n  query DevicePolicy($id: ID!) {\n    device(id: $id) {\n      sessionLimit\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation SetDevicePolicy($id: ID!, $input: UpdateDeviceInput!) {\n    updateDevice(id: $id, input: $input) {\n      sessionLimit\n    }\n  }\n"): (typeof documents)["\n  mutation SetDevicePolicy($id: ID!, $input: UpdateDeviceInput!) {\n    updateDevice(id: $id, input: $input) {\n      sessionLimit\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Events($filter: EventFilter, $first: Int, $after: String) {\n    events(filter: $filter, first: $first, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          at\n          severity\n          source\n          deviceId\n          deviceName\n          message\n        }\n      }\n    }\n  }\n"): (typeof documents)["\n  query Events($filter: EventFilter, $first: Int, $after: String) {\n    events(filter: $filter, first: $first, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          at\n          severity\n          source\n          deviceId\n          deviceName\n          message\n        }\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  subscription EventAdded($minSeverity: Severity) {\n    eventAdded(minSeverity: $minSeverity) {\n      id\n      at\n      severity\n      source\n      deviceId\n      deviceName\n      message\n    }\n  }\n"): (typeof documents)["\n  subscription EventAdded($minSeverity: Severity) {\n    eventAdded(minSeverity: $minSeverity) {\n      id\n      at\n      severity\n      source\n      deviceId\n      deviceName\n      message\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query DeviceMetrics($id: ID!, $range: MetricRange!) {\n    device(id: $id) {\n      id\n      metricsSource\n      gpus {\n        index\n        name\n        vendor\n        hasVram\n      }\n      readings {\n        at\n        cpuPercent\n        memoryPercent\n        diskPercent\n        diskMount\n        otherDrives\n        vramPercent\n        vramGpu {\n          index\n          name\n          vendor\n          hasVram\n        }\n      }\n      storage {\n        drives {\n          name\n          kind\n          sizeBytes\n          mounted\n          mounts {\n            path\n            usedBytes\n            sizeBytes\n            percent\n          }\n          readBps\n          writeBps\n          busyPercent\n        }\n      }\n      metrics(range: $range) {\n        kind\n        label\n        gpuIndex\n        points {\n          at\n          value\n          min\n          max\n        }\n      }\n    }\n  }\n"): (typeof documents)["\n  query DeviceMetrics($id: ID!, $range: MetricRange!) {\n    device(id: $id) {\n      id\n      metricsSource\n      gpus {\n        index\n        name\n        vendor\n        hasVram\n      }\n      readings {\n        at\n        cpuPercent\n        memoryPercent\n        diskPercent\n        diskMount\n        otherDrives\n        vramPercent\n        vramGpu {\n          index\n          name\n          vendor\n          hasVram\n        }\n      }\n      storage {\n        drives {\n          name\n          kind\n          sizeBytes\n          mounted\n          mounts {\n            path\n            usedBytes\n            sizeBytes\n            percent\n          }\n          readBps\n          writeBps\n          busyPercent\n        }\n      }\n      metrics(range: $range) {\n        kind\n        label\n        gpuIndex\n        points {\n          at\n          value\n          min\n          max\n        }\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  subscription MetricsAdded($deviceId: ID!) {\n    metricsAdded(deviceId: $deviceId) {\n      deviceId\n      at\n      samples {\n        kind\n        label\n        value\n      }\n      readings {\n        at\n        cpuPercent\n        memoryPercent\n        diskPercent\n        diskMount\n        otherDrives\n        vramPercent\n        vramGpu {\n          index\n          name\n          vendor\n          hasVram\n        }\n      }\n    }\n  }\n"): (typeof documents)["\n  subscription MetricsAdded($deviceId: ID!) {\n    metricsAdded(deviceId: $deviceId) {\n      deviceId\n      at\n      samples {\n        kind\n        label\n        value\n      }\n      readings {\n        at\n        cpuPercent\n        memoryPercent\n        diskPercent\n        diskMount\n        otherDrives\n        vramPercent\n        vramGpu {\n          index\n          name\n          vendor\n          hasVram\n        }\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query AgentStatus($id: ID!) {\n    device(id: $id) {\n      id\n      metricsSource\n      metricsStale\n    }\n  }\n"): (typeof documents)["\n  query AgentStatus($id: ID!) {\n    device(id: $id) {\n      id\n      metricsSource\n      metricsStale\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation EnrolAgent($deviceId: ID!) {\n    enrolAgent(deviceId: $deviceId) {\n      code\n      expiresAt\n    }\n  }\n"): (typeof documents)["\n  mutation EnrolAgent($deviceId: ID!) {\n    enrolAgent(deviceId: $deviceId) {\n      code\n      expiresAt\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation RevokeAgentToken($deviceId: ID!) {\n    revokeAgentToken(deviceId: $deviceId) {\n      id\n      metricsSource\n      metricsStale\n    }\n  }\n"): (typeof documents)["\n  mutation RevokeAgentToken($deviceId: ID!) {\n    revokeAgentToken(deviceId: $deviceId) {\n      id\n      metricsSource\n      metricsStale\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment DeliveryFields on Delivery {\n    id\n    channelId\n    kind\n    at\n    ok\n    reason\n    code\n  }\n"): (typeof documents)["\n  fragment DeliveryFields on Delivery {\n    id\n    channelId\n    kind\n    at\n    ok\n    reason\n    code\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment NotificationChannelFields on NotificationChannel {\n    id\n    name\n    kind\n    onByDefault\n    settings {\n      __typename\n      ... on WebhookSettings {\n        origin\n        method\n        headerNames\n      }\n      ... on NtfySettings {\n        server\n        priority\n        tokenStored\n      }\n      ... on EmailSettings {\n        host\n        port\n        security\n        username\n        from\n        to\n        passwordStored\n      }\n      ... on AppriseSettings {\n        server\n        mode\n        tags\n      }\n    }\n    template {\n      title\n      body\n    }\n    lastDelivery {\n      ...DeliveryFields\n    }\n  }\n"): (typeof documents)["\n  fragment NotificationChannelFields on NotificationChannel {\n    id\n    name\n    kind\n    onByDefault\n    settings {\n      __typename\n      ... on WebhookSettings {\n        origin\n        method\n        headerNames\n      }\n      ... on NtfySettings {\n        server\n        priority\n        tokenStored\n      }\n      ... on EmailSettings {\n        host\n        port\n        security\n        username\n        from\n        to\n        passwordStored\n      }\n      ... on AppriseSettings {\n        server\n        mode\n        tags\n      }\n    }\n    template {\n      title\n      body\n    }\n    lastDelivery {\n      ...DeliveryFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment NotificationSettingsFields on NotificationSettings {\n    certificateDays\n    template {\n      title\n      body\n    }\n    placeholders\n  }\n"): (typeof documents)["\n  fragment NotificationSettingsFields on NotificationSettings {\n    certificateDays\n    template {\n      title\n      body\n    }\n    placeholders\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query NotificationChannels {\n    notificationChannels {\n      ...NotificationChannelFields\n    }\n  }\n"): (typeof documents)["\n  query NotificationChannels {\n    notificationChannels {\n      ...NotificationChannelFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query NotificationSettings {\n    notificationSettings {\n      ...NotificationSettingsFields\n    }\n  }\n"): (typeof documents)["\n  query NotificationSettings {\n    notificationSettings {\n      ...NotificationSettingsFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CreateNotificationChannel($input: NotificationChannelInput!) {\n    createNotificationChannel(input: $input) {\n      ...NotificationChannelFields\n    }\n  }\n"): (typeof documents)["\n  mutation CreateNotificationChannel($input: NotificationChannelInput!) {\n    createNotificationChannel(input: $input) {\n      ...NotificationChannelFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UpdateNotificationChannel($id: ID!, $input: NotificationChannelInput!) {\n    updateNotificationChannel(id: $id, input: $input) {\n      ...NotificationChannelFields\n    }\n  }\n"): (typeof documents)["\n  mutation UpdateNotificationChannel($id: ID!, $input: NotificationChannelInput!) {\n    updateNotificationChannel(id: $id, input: $input) {\n      ...NotificationChannelFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation DeleteNotificationChannel($id: ID!) {\n    deleteNotificationChannel(id: $id)\n  }\n"): (typeof documents)["\n  mutation DeleteNotificationChannel($id: ID!) {\n    deleteNotificationChannel(id: $id)\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation SendTestNotification($id: ID!) {\n    sendTestNotification(id: $id) {\n      ...DeliveryFields\n    }\n  }\n"): (typeof documents)["\n  mutation SendTestNotification($id: ID!) {\n    sendTestNotification(id: $id) {\n      ...DeliveryFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation SetNotificationSettings($input: NotificationSettingsInput!) {\n    setNotificationSettings(input: $input) {\n      ...NotificationSettingsFields\n    }\n  }\n"): (typeof documents)["\n  mutation SetNotificationSettings($input: NotificationSettingsInput!) {\n    setNotificationSettings(input: $input) {\n      ...NotificationSettingsFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Retention {\n    retention {\n      subject\n      days\n      minDays\n      maxDays\n      defaultDays\n    }\n  }\n"): (typeof documents)["\n  query Retention {\n    retention {\n      subject\n      days\n      minDays\n      maxDays\n      defaultDays\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation SetRetention($subject: RetentionSubject!, $days: Int!) {\n    setRetention(subject: $subject, days: $days) {\n      subject\n      days\n      minDays\n      maxDays\n      defaultDays\n    }\n  }\n"): (typeof documents)["\n  mutation SetRetention($subject: RetentionSubject!, $days: Int!) {\n    setRetention(subject: $subject, days: $days) {\n      subject\n      days\n      minDays\n      maxDays\n      defaultDays\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation StartSession($input: StartSessionInput!) {\n    startSession(input: $input) {\n      sessionId\n      token\n      tunnelPath\n      expiresAt\n      wakeSent\n    }\n  }\n"): (typeof documents)["\n  mutation StartSession($input: StartSessionInput!) {\n    startSession(input: $input) {\n      sessionId\n      token\n      tunnelPath\n      expiresAt\n      wakeSent\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation EndSession($id: ID!) {\n    endSession(id: $id) {\n      id\n      deviceId\n      deviceName\n      protocol\n      startedAt\n      endedAt\n      endReason\n    }\n  }\n"): (typeof documents)["\n  mutation EndSession($id: ID!) {\n    endSession(id: $id) {\n      id\n      deviceId\n      deviceName\n      protocol\n      startedAt\n      endedAt\n      endReason\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation TestConnection($input: ConnectionTestInput!) {\n    testConnection(input: $input) {\n      reachable\n      authenticated\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n      hostKeyChanged\n      message\n    }\n  }\n"): (typeof documents)["\n  mutation TestConnection($input: ConnectionTestInput!) {\n    testConnection(input: $input) {\n      reachable\n      authenticated\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n      hostKeyChanged\n      message\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation AcceptHostKey($deviceId: ID!, $fingerprintSha256: String!) {\n    acceptHostKey(deviceId: $deviceId, fingerprintSha256: $fingerprintSha256) {\n      algorithm\n      fingerprintSha256\n      firstSeenAt\n    }\n  }\n"): (typeof documents)["\n  mutation AcceptHostKey($deviceId: ID!, $fingerprintSha256: String!) {\n    acceptHostKey(deviceId: $deviceId, fingerprintSha256: $fingerprintSha256) {\n      algorithm\n      fingerprintSha256\n      firstSeenAt\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation TestRemoteConnection($input: ConnectionTestInput!) {\n    testConnection(input: $input) {\n      reachable\n      authenticated\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n      hostKeyChanged\n      message\n      serverCertificate {\n        fingerprintSha256\n        subject\n        notAfter\n        firstSeenAt\n      }\n      certificateChanged\n      certificateMatched\n    }\n  }\n"): (typeof documents)["\n  mutation TestRemoteConnection($input: ConnectionTestInput!) {\n    testConnection(input: $input) {\n      reachable\n      authenticated\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n      hostKeyChanged\n      message\n      serverCertificate {\n        fingerprintSha256\n        subject\n        notAfter\n        firstSeenAt\n      }\n      certificateChanged\n      certificateMatched\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation AcceptServerCertificate($deviceId: ID!, $fingerprintSha256: String!) {\n    acceptServerCertificate(deviceId: $deviceId, fingerprintSha256: $fingerprintSha256) {\n      fingerprintSha256\n      subject\n      notAfter\n      firstSeenAt\n    }\n  }\n"): (typeof documents)["\n  mutation AcceptServerCertificate($deviceId: ID!, $fingerprintSha256: String!) {\n    acceptServerCertificate(deviceId: $deviceId, fingerprintSha256: $fingerprintSha256) {\n      fingerprintSha256\n      subject\n      notAfter\n      firstSeenAt\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query DeviceTransfer($id: ID!) {\n    device(id: $id) {\n      transfer {\n        copy\n        paste\n      }\n    }\n  }\n"): (typeof documents)["\n  query DeviceTransfer($id: ID!) {\n    device(id: $id) {\n      transfer {\n        copy\n        paste\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation SetRdpCredential($deviceId: ID!, $credential: PasswordCredentialInput!) {\n    setRdpCredential(deviceId: $deviceId, credential: $credential) {\n      credentialStored\n    }\n  }\n"): (typeof documents)["\n  mutation SetRdpCredential($deviceId: ID!, $credential: PasswordCredentialInput!) {\n    setRdpCredential(deviceId: $deviceId, credential: $credential) {\n      credentialStored\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation SetVncCredential($deviceId: ID!, $credential: PasswordCredentialInput!) {\n    setVncCredential(deviceId: $deviceId, credential: $credential) {\n      credentialStored\n    }\n  }\n"): (typeof documents)["\n  mutation SetVncCredential($deviceId: ID!, $credential: PasswordCredentialInput!) {\n    setVncCredential(deviceId: $deviceId, credential: $credential) {\n      credentialStored\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query DeviceSessions($deviceId: ID!, $first: Int, $after: String) {\n    sessions(deviceId: $deviceId, first: $first, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          protocol\n          operatorName\n          startedAt\n          endedAt\n          endReason\n        }\n      }\n    }\n  }\n"): (typeof documents)["\n  query DeviceSessions($deviceId: ID!, $first: Int, $after: String) {\n    sessions(deviceId: $deviceId, first: $first, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          protocol\n          operatorName\n          startedAt\n          endedAt\n          endReason\n        }\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation OpenFiles($sessionId: ID!) {\n    openFiles(sessionId: $sessionId) {\n      sessionId\n      token\n      tunnelPath\n      expiresAt\n    }\n  }\n"): (typeof documents)["\n  mutation OpenFiles($sessionId: ID!) {\n    openFiles(sessionId: $sessionId) {\n      sessionId\n      token\n      tunnelPath\n      expiresAt\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query LiveSessions($first: Int) {\n    sessions(live: true, first: $first) {\n      edges {\n        node {\n          id\n          deviceId\n          deviceName\n          protocol\n          operatorName\n          startedAt\n          clientAddr\n        }\n      }\n    }\n  }\n"): (typeof documents)["\n  query LiveSessions($first: Int) {\n    sessions(live: true, first: $first) {\n      edges {\n        node {\n          id\n          deviceId\n          deviceName\n          protocol\n          operatorName\n          startedAt\n          clientAddr\n        }\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  subscription LiveSessionChanged {\n    sessionChanged {\n      session {\n        id\n        deviceId\n        deviceName\n        protocol\n        operatorName\n        startedAt\n        clientAddr\n        endedAt\n        endReason\n      }\n    }\n  }\n"): (typeof documents)["\n  subscription LiveSessionChanged {\n    sessionChanged {\n      session {\n        id\n        deviceId\n        deviceName\n        protocol\n        operatorName\n        startedAt\n        clientAddr\n        endedAt\n        endReason\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation WakeDevice($deviceId: ID!) {\n    wakeDevice(deviceId: $deviceId) {\n      sentAt\n      relayName\n    }\n  }\n"): (typeof documents)["\n  mutation WakeDevice($deviceId: ID!) {\n    wakeDevice(deviceId: $deviceId) {\n      sentAt\n      relayName\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment VpnNodeFields on VpnNode {\n    name\n    address\n    os\n    role\n    tags\n    routes\n    waitingRoutes\n    online\n    lastSeen\n    keyExpiry\n    keyState\n    path\n    relayRegion\n    inAnima\n    device {\n      id\n      name\n    }\n  }\n"): (typeof documents)["\n  fragment VpnNodeFields on VpnNode {\n    name\n    address\n    os\n    role\n    tags\n    routes\n    waitingRoutes\n    online\n    lastSeen\n    keyExpiry\n    keyState\n    path\n    relayRegion\n    inAnima\n    device {\n      id\n      name\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment VpnPictureFields on VpnPicture {\n    readAt\n    fresh\n    hub {\n      ...VpnNodeFields\n    }\n    sites {\n      site {\n        id\n        name\n        subnet\n      }\n      router {\n        ...VpnNodeFields\n      }\n      state\n      path\n      relayRegion\n      hasPingCheck\n      waitingRouter {\n        ...VpnNodeFields\n      }\n    }\n    nodes {\n      ...VpnNodeFields\n    }\n    broker\n  }\n"): (typeof documents)["\n  fragment VpnPictureFields on VpnPicture {\n    readAt\n    fresh\n    hub {\n      ...VpnNodeFields\n    }\n    sites {\n      site {\n        id\n        name\n        subnet\n      }\n      router {\n        ...VpnNodeFields\n      }\n      state\n      path\n      relayRegion\n      hasPingCheck\n      waitingRouter {\n        ...VpnNodeFields\n      }\n    }\n    nodes {\n      ...VpnNodeFields\n    }\n    broker\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Vpn {\n    vpn {\n      ...VpnPictureFields\n    }\n  }\n"): (typeof documents)["\n  query Vpn {\n    vpn {\n      ...VpnPictureFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  subscription VpnChanged {\n    vpnChanged {\n      ...VpnPictureFields\n    }\n  }\n"): (typeof documents)["\n  subscription VpnChanged {\n    vpnChanged {\n      ...VpnPictureFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CreateVpnInvite {\n    createVpnInvite {\n      key\n      expiresAt\n      steps\n    }\n  }\n"): (typeof documents)["\n  mutation CreateVpnInvite {\n    createVpnInvite {\n      key\n      expiresAt\n      steps\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CreateRouterKey($siteId: ID!) {\n    createRouterKey(siteId: $siteId) {\n      key\n      expiresAt\n      steps\n    }\n  }\n"): (typeof documents)["\n  mutation CreateRouterKey($siteId: ID!) {\n    createRouterKey(siteId: $siteId) {\n      key\n      expiresAt\n      steps\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation RegisterVpnNode($authId: String!) {\n    registerVpnNode(authId: $authId) {\n      name\n      address\n      keyExpiry\n    }\n  }\n"): (typeof documents)["\n  mutation RegisterVpnNode($authId: String!) {\n    registerVpnNode(authId: $authId) {\n      name\n      address\n      keyExpiry\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation RenewVpnKey($name: String!) {\n    renewVpnKey(name: $name) {\n      name\n      address\n      keyExpiry\n    }\n  }\n"): (typeof documents)["\n  mutation RenewVpnKey($name: String!) {\n    renewVpnKey(name: $name) {\n      name\n      address\n      keyExpiry\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation ApproveSiteRoute($siteId: ID!) {\n    approveSiteRoute(siteId: $siteId)\n  }\n"): (typeof documents)["\n  mutation ApproveSiteRoute($siteId: ID!) {\n    approveSiteRoute(siteId: $siteId)\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment WebInterfaceSuggestionFields on WebInterfaceSuggestion {\n    checkId\n    url\n    label\n  }\n"): (typeof documents)["\n  fragment WebInterfaceSuggestionFields on WebInterfaceSuggestion {\n    checkId\n    url\n    label\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment WebInterfaceFields on WebInterface {\n    id\n    deviceId\n    label\n    url\n    openIn\n    pinnedFingerprintSha256\n    pinnedAt\n  }\n"): (typeof documents)["\n  fragment WebInterfaceFields on WebInterface {\n    id\n    deviceId\n    label\n    url\n    openIn\n    pinnedFingerprintSha256\n    pinnedAt\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CreateWebInterface($deviceId: ID!, $input: WebInterfaceInput!) {\n    createWebInterface(deviceId: $deviceId, input: $input) {\n      ...WebInterfaceFields\n    }\n  }\n"): (typeof documents)["\n  mutation CreateWebInterface($deviceId: ID!, $input: WebInterfaceInput!) {\n    createWebInterface(deviceId: $deviceId, input: $input) {\n      ...WebInterfaceFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UpdateWebInterface($id: ID!, $input: WebInterfaceInput!) {\n    updateWebInterface(id: $id, input: $input) {\n      ...WebInterfaceFields\n    }\n  }\n"): (typeof documents)["\n  mutation UpdateWebInterface($id: ID!, $input: WebInterfaceInput!) {\n    updateWebInterface(id: $id, input: $input) {\n      ...WebInterfaceFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation DeleteWebInterface($id: ID!) {\n    deleteWebInterface(id: $id)\n  }\n"): (typeof documents)["\n  mutation DeleteWebInterface($id: ID!) {\n    deleteWebInterface(id: $id)\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation AcceptWebInterfaceSuggestion($checkId: ID!) {\n    acceptWebInterfaceSuggestion(checkId: $checkId) {\n      ...WebInterfaceFields\n    }\n  }\n"): (typeof documents)["\n  mutation AcceptWebInterfaceSuggestion($checkId: ID!) {\n    acceptWebInterfaceSuggestion(checkId: $checkId) {\n      ...WebInterfaceFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation DismissWebInterfaceSuggestion($checkId: ID!) {\n    dismissWebInterfaceSuggestion(checkId: $checkId)\n  }\n"): (typeof documents)["\n  mutation DismissWebInterfaceSuggestion($checkId: ID!) {\n    dismissWebInterfaceSuggestion(checkId: $checkId)\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation AcceptWebCertificate($id: ID!, $fingerprintSha256: String!) {\n    acceptWebCertificate(id: $id, fingerprintSha256: $fingerprintSha256) {\n      id\n      pinnedFingerprintSha256\n      pinnedAt\n    }\n  }\n"): (typeof documents)["\n  mutation AcceptWebCertificate($id: ID!, $fingerprintSha256: String!) {\n    acceptWebCertificate(id: $id, fingerprintSha256: $fingerprintSha256) {\n      id\n      pinnedFingerprintSha256\n      pinnedAt\n    }\n  }\n"];

export function graphql(source: string) {
  return (documents as any)[source] ?? {};
}

export type DocumentType<TDocumentNode extends DocumentNode<any, any>> = TDocumentNode extends DocumentNode<  infer TType,  any>  ? TType  : never;