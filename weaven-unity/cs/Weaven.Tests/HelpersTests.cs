using System.Collections.Generic;
using Xunit;
using Weaven;

namespace Weaven.Tests
{
    public class HelpersTests
    {
        // ── ParseUintArray ──────────────────────────────────────────────

        [Fact]
        public void ParseUintArray_EmptyArray_ReturnsEmpty()
        {
            var result = WeavenWorld.ParseUintArray("[]");
            Assert.Empty(result);
        }

        [Fact]
        public void ParseUintArray_SingleElement()
        {
            var result = WeavenWorld.ParseUintArray("[42]");
            Assert.Equal(new uint[] { 42 }, result);
        }

        [Fact]
        public void ParseUintArray_MultipleElements()
        {
            var result = WeavenWorld.ParseUintArray("[1,2,3]");
            Assert.Equal(new uint[] { 1, 2, 3 }, result);
        }

        [Fact]
        public void ParseUintArray_WithWhitespace()
        {
            var result = WeavenWorld.ParseUintArray("[ 1 , 2 , 3 ]");
            Assert.Equal(new uint[] { 1, 2, 3 }, result);
        }

        [Fact]
        public void ParseUintArray_LeadingTrailingWhitespace()
        {
            var result = WeavenWorld.ParseUintArray("  [10,20]  ");
            Assert.Equal(new uint[] { 10, 20 }, result);
        }

        // ── DictToJson ──────────────────────────────────────────────────

        [Fact]
        public void DictToJson_EmptyDict_ReturnsEmptyObject()
        {
            var result = WeavenWorld.DictToJson(new Dictionary<string, double>());
            Assert.Equal("{}", result);
        }

        [Fact]
        public void DictToJson_SingleEntry()
        {
            var dict = new Dictionary<string, double> { { "speed", 3.0 } };
            var result = WeavenWorld.DictToJson(dict);
            Assert.Contains("\"speed\":", result);
            Assert.StartsWith("{", result);
            Assert.EndsWith("}", result);
        }

        [Fact]
        public void DictToJson_SpecialCharacters_Escaped()
        {
            var dict = new Dictionary<string, double> { { "key\"with\\quotes", 1.0 } };
            var result = WeavenWorld.DictToJson(dict);
            Assert.Contains("key\\\"with\\\\quotes", result);
        }

        // ── UintArrayToJson ─────────────────────────────────────────────

        [Fact]
        public void UintArrayToJson_EmptyArray()
        {
            var result = WeavenWorld.UintArrayToJson(System.Array.Empty<uint>());
            Assert.Equal("[]", result);
        }

        [Fact]
        public void UintArrayToJson_SingleElement()
        {
            var result = WeavenWorld.UintArrayToJson(new uint[] { 42 });
            Assert.Equal("[42]", result);
        }

        [Fact]
        public void UintArrayToJson_MultipleElements()
        {
            var result = WeavenWorld.UintArrayToJson(new uint[] { 1, 2, 3 });
            Assert.Equal("[1,2,3]", result);
        }

        // ── Escape ──────────────────────────────────────────────────────

        [Fact]
        public void Escape_NoSpecialChars_Unchanged()
        {
            Assert.Equal("hello", WeavenWorld.Escape("hello"));
        }

        [Fact]
        public void Escape_BackslashEscaped()
        {
            Assert.Equal("a\\\\b", WeavenWorld.Escape("a\\b"));
        }

        [Fact]
        public void Escape_QuoteEscaped()
        {
            Assert.Equal("a\\\"b", WeavenWorld.Escape("a\"b"));
        }

        [Fact]
        public void Escape_BothBackslashAndQuote()
        {
            Assert.Equal("\\\\\\\"", WeavenWorld.Escape("\\\""));
        }

        [Fact]
        public void Escape_EmptyString()
        {
            Assert.Equal("", WeavenWorld.Escape(""));
        }

        // ── Roundtrip: UintArrayToJson → ParseUintArray ─────────────────

        [Fact]
        public void UintArray_Roundtrip()
        {
            var original = new uint[] { 10, 20, 30, 4294967295 };
            var json = WeavenWorld.UintArrayToJson(original);
            var parsed = WeavenWorld.ParseUintArray(json);
            Assert.Equal(original, parsed);
        }
    }
}
