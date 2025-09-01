using UnityEngine;
using Pb = Mcp.Unity.V1;
using System;

namespace Mcp.Unity.V1.Ipc
{
    internal static class ComponentsHandler
    {
        private static Type FindType(string name)
        {
            var type = Type.GetType(name);
            if (type == null)
            {
                type = Type.GetType($"UnityEngine.{name}, UnityEngine");
            }
            return type;
        }

        public static Pb.ComponentResponse Handle(Pb.ComponentRequest req, Bridge.Editor.Ipc.FeatureGuard features)
        {
            switch (req.PayloadCase)
            {
                case Pb.ComponentRequest.PayloadOneofCase.Add:
                    return Add(req.Add);
                case Pb.ComponentRequest.PayloadOneofCase.Get:
                    return Get(req.Get);
                case Pb.ComponentRequest.PayloadOneofCase.Remove:
                    return Remove(req.Remove);
                default:
                    return new Pb.ComponentResponse { StatusCode = 2, Message = "invalid request" };
            }
        }

        private static Pb.ComponentResponse Add(Pb.AddComponentRequest r)
        {
            var go = GameObject.Find(r.GameObject);
            if (go == null)
                return new Pb.ComponentResponse { StatusCode = 5, Message = "game object not found" };

            var type = FindType(r.Component);
            if (type == null || !typeof(Component).IsAssignableFrom(type))
                return new Pb.ComponentResponse { StatusCode = 2, Message = "invalid component type" };

            try
            {
                go.AddComponent(type);
                return new Pb.ComponentResponse { StatusCode = 0, Add = new Pb.AddComponentResponse { Ok = true } };
            }
            catch (Exception ex)
            {
                return new Pb.ComponentResponse { StatusCode = 13, Message = ex.Message };
            }
        }

        private static Pb.ComponentResponse Get(Pb.GetComponentsRequest r)
        {
            var go = GameObject.Find(r.GameObject);
            if (go == null)
                return new Pb.ComponentResponse { StatusCode = 5, Message = "game object not found" };

            var comps = go.GetComponents<Component>();
            var resp = new Pb.GetComponentsResponse();
            foreach (var c in comps)
            {
                resp.Components.Add(c.GetType().FullName);
            }
            return new Pb.ComponentResponse { StatusCode = 0, Get = resp };
        }

        private static Pb.ComponentResponse Remove(Pb.RemoveComponentRequest r)
        {
            var go = GameObject.Find(r.GameObject);
            if (go == null)
                return new Pb.ComponentResponse { StatusCode = 5, Message = "game object not found" };

            var type = FindType(r.Component);
            if (type == null || !typeof(Component).IsAssignableFrom(type))
                return new Pb.ComponentResponse { StatusCode = 2, Message = "invalid component type" };

            var comp = go.GetComponent(type);
            if (comp == null)
                return new Pb.ComponentResponse { StatusCode = 5, Message = "component not found" };

            UnityEngine.Object.DestroyImmediate(comp);
            return new Pb.ComponentResponse { StatusCode = 0, Remove = new Pb.RemoveComponentResponse { Ok = true } };
        }
    }
}
