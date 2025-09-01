#if UNITY_EDITOR
using System;
using System.Net;
using NUnit.Framework;
using UnityEngine;
using Mcp.Unity.V1.Ipc;

namespace Mcp.Unity.V1.Ipc.Tests
{
    /// <summary>
    /// Tests for TcpTransport port configuration functionality
    /// This validates the BDD scenario: Unity側ポート番号の動的設定
    /// </summary>
    [TestFixture]
    public class TcpTransportPortTests
    {
        [Test]
        public void CreateDefault_ShouldUsePort7777()
        {
            // Given: No port configuration specified
            // When: Creating default TcpTransport
            var transport = TcpTransport.CreateDefault();
            
            // Then: Should use port 7777
            // This test should PASS initially (current behavior)
            Assert.IsNotNull(transport, "Default transport should be created");
            // Note: We can't easily access the internal port without starting the transport
            // This test validates that CreateDefault() works as expected
        }

        [Test] 
        public void Constructor_ShouldAcceptCustomPort()
        {
            // Given: Port number 8888 specified
            var customEndpoint = new IPEndPoint(IPAddress.Loopback, 8888);
            
            // When: Creating TcpTransport with custom port
            var transport = new TcpTransport(customEndpoint);
            
            // Then: Transport should be created successfully
            // This test should PASS initially - basic constructor works
            Assert.IsNotNull(transport, "Custom port transport should be created");
            transport.Dispose();
        }

        [Test]
        public void CreateWithPort_ShouldUseSpecifiedPort() 
        {
            // GREEN Phase: Test the new CreateWithPort functionality
            
            // Given: Port number 9999 specified
            const int customPort = 9999;
            
            // When: Creating transport with specified port
            var transport = TcpTransport.CreateWithPort(customPort);
            
            // Then: Should use port 9999
            Assert.IsNotNull(transport, "Custom port transport should be created");
            Assert.AreEqual(customPort, transport.Port, "Transport should use the specified port");
            Assert.AreEqual(IPAddress.Loopback.ToString(), transport.Endpoint.Address.ToString(), 
                "Transport should use loopback address");
            
            transport.Dispose();
        }

        [Test]
        public void CreateWithCustomPort_ShouldStartOnSpecifiedPort()
        {
            // GREEN Phase: Now we can properly verify the port configuration
            
            // Given: A custom port number
            const int customPort = 8888;
            var endpoint = new IPEndPoint(IPAddress.Loopback, customPort);
            var transport = new TcpTransport(endpoint);
            
            try
            {
                // When: Starting the transport
                transport.Start();
                
                // Then: We can verify it's using the correct port
                Assert.IsTrue(transport.IsListening, "Transport should be listening");
                Assert.AreEqual(customPort, transport.Port, "Transport should use the specified port");
                
                // Additional verification: the endpoint should match what we configured
                Assert.AreEqual(customPort, transport.Endpoint.Port, "Endpoint port should match");
                Assert.AreEqual(IPAddress.Loopback, transport.Endpoint.Address, "Endpoint address should be loopback");
                
            }
            finally
            {
                transport.Stop();
                transport.Dispose();
            }
        }

        [Test]
        public void DefaultTransport_ShouldUsePort7777()
        {
            // Additional test to ensure default behavior is preserved
            
            // Given: Default transport creation
            var transport = TcpTransport.CreateDefault();
            
            // When: Checking configured port
            var configuredPort = transport.Port;
            
            // Then: Should be 7777
            Assert.AreEqual(7777, configuredPort, "Default transport should use port 7777");
            
            transport.Dispose();
        }
    }
}
#endif