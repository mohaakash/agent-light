import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

export async function hasNotificationPermission(): Promise<boolean> {
  try {
    return await isPermissionGranted();
  } catch {
    return false;
  }
}

export async function requestNotificationPermission(): Promise<boolean> {
  try {
    if (await isPermissionGranted()) return true;
    return (await requestPermission()) === "granted";
  } catch {
    return false;
  }
}

export async function sendDesktopNotification(title: string, body: string, enabled: boolean): Promise<void> {
  if (!enabled) return;
  try {
    if (!(await isPermissionGranted())) return;
    sendNotification({
      title,
      body,
      group: "agentlight-attention",
      autoCancel: true,
      silent: true,
    });
  } catch {
    // Audio alerts remain available when the OS notification service is unavailable.
  }
}
