// Unity MCP Bridge - Project Settings Handler
// Handles project settings get/set via IPC
using System;
using UnityEditor;
using UnityEngine;
using Pb = Mcp.Unity.V1;

namespace Mcp.Unity.V1.Ipc
{
    internal static class ProjectSettingsHandler
    {
        public static Pb.GetProjectSettingsResponse HandleGet(Pb.GetProjectSettingsRequest request)
        {
            var response = new Pb.GetProjectSettingsResponse
            {
                Success = true,
                ErrorMessage = ""
            };

            foreach (var key in request.Keys)
            {
                switch (key)
                {
                    case "companyName":
                        response.Settings.Add(key, PlayerSettings.companyName);
                        break;
                    case "productName":
                        response.Settings.Add(key, PlayerSettings.productName);
                        break;
                }
            }

            return response;
        }

        public static Pb.SetProjectSettingsResponse HandleSet(Pb.SetProjectSettingsRequest request)
        {
            try
            {
                foreach (var kv in request.Settings)
                {
                    switch (kv.Key)
                    {
                        case "companyName":
                            PlayerSettings.companyName = kv.Value;
                            break;
                        case "productName":
                            PlayerSettings.productName = kv.Value;
                            break;
                    }
                }

                return new Pb.SetProjectSettingsResponse { Ok = true, ErrorMessage = "" };
            }
            catch (Exception ex)
            {
                Debug.LogError($"[ProjectSettingsHandler] Failed to set project settings: {ex.Message}");
                return new Pb.SetProjectSettingsResponse { Ok = false, ErrorMessage = ex.Message };
            }
        }
    }
}
