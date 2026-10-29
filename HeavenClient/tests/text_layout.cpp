#include "../Graphics/TextLayout.h"
#include "../Graphics/Utf8.h"
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

namespace
{
	// 6 px per character, the ellipsis 12, anything else 7; bold adds 1.
	int16_t advance(char32_t c, bool bold)
	{
		if (c == 0x2026)
			return 12 + bold;

		return (c < 128 ? 6 : 7) + bold;
	}

	ms::Text::Layout layout(const std::string &text, int16_t maxwidth, const std::vector<ms::Point<int16_t>> &boxes = {})
	{
		return ms::layout_text(text, 15, advance, ms::Text::Alignment::LEFT, maxwidth, true, 0, boxes);
	}

	size_t line_count(const ms::Text::Layout &l)
	{
		size_t count = 0;

		for (auto it = l.begin(); it != l.end(); ++it)
			count++;

		return count;
	}
}

int main()
{
	try
	{
		char32_t c;
		const std::string puro = "Nothing to do\xE2\x80\xA6 lalala.";
		require(ms::Utf8::decode(puro, 13, c) == 3 && c == 0x2026, "ellipsis decodes");
		require(ms::Utf8::decode(puro, 14, c) == 0, "continuation bytes have no character");
		require(ms::Utf8::decode("\xC3\xA9", 0, c) == 2 && c == 0xE9, "two-byte character");
		require(ms::Utf8::decode("\xE2\x80", 0, c) == 1 && c == ms::Utf8::REPLACEMENT, "truncated sequence");
		require(ms::Utf8::decode("\xFF" "a", 0, c) == 1 && c == ms::Utf8::REPLACEMENT, "invalid lead byte");

		// Issue #495: Puro's balloon line, 84 px wide.
		auto l = layout(puro, 84);
		require(line_count(l) == 2, "\" do\xE2\x80\xA6\" moves to the second line");
		require(l.advance(13) == 12 && l.advance(14) == 24 && l.advance(15) == 24 && l.advance(16) == 24,
			"the ellipsis advances once, its later bytes take no room");

		// Characters wider than the line still make progress.
		auto narrow = layout("\xE2\x80\xA6\xE2\x80\xA6", 5);
		require(line_count(narrow) == 2, "each too-wide character takes its own line");
		auto ascii = layout("abc", 3);
		require(line_count(ascii) == 3 && ascii.advance(1) == 0 && ascii.advance(2) == 0,
			"each too-wide ASCII character takes its own line");
		auto box = layout("\x01" "ab", 12, {{40, 10}});
		require(line_count(box) == 2 && box.box(0).x() == 0, "a box wider than the line sits alone");
		require(layout("a", 3).advance(1) == 6, "a lone character wider than the line is still placed");

		// #e makes text bold until #n: wider, and drawn in the bold font.
		auto bold = layout("#ea b#nc", 100);
		auto bold_at = [&bold](size_t pos)
		{
			for (const auto &line : bold)
				for (const auto &word : line.words)
					if (word.first <= pos && pos < word.last)
						return word.bold;

			throw std::runtime_error("no word holds the character");
		};
		require(bold.advance(4) == 14 && bold.advance(7) == 21 && bold.width() == 27, "bold characters are wider");
		require(bold_at(2) && bold_at(4) && !bold_at(7), "words between #e and #n are bold");

		// A '#' after #c ends the orange run and skips only itself, as in
		// Blessing of the Fairy's "#c%s#'s energy" (v83 0x8e4217).
		const std::string blessing = "of #cSib#'s energy";
		auto tip = layout(blessing, 200);
		auto word_at = [&tip](size_t pos)
		{
			for (const auto &line : tip)
				for (const auto &word : line.words)
					if (word.first <= pos && pos < word.last)
						return word;

			throw std::runtime_error("no word holds the character");
		};
		size_t apostrophe = blessing.find('\'');
		require(word_at(5).color == ms::Color::Name::ORANGE, "#c turns the name orange");
		require(word_at(apostrophe).color == ms::Color::Name::NUM_COLORS, "the closing # ends the orange");
		require(tip.advance(apostrophe + 1) - tip.advance(apostrophe) == 6, "the apostrophe is drawn");
		require(word_at(blessing.find('e')).color == ms::Color::Name::NUM_COLORS, "the words after stay plain");

		// Finding the character under the cursor, as for a name in the quest log.
		auto words = layout("abc def", 24);
		require(words.index_at({ 7, 10 }) == 1, "the b of the first line");
		require(words.index_at({ 1, 25 }) == 4, "the d starts the second line");
		require(words.index_at({ 13, 33 }) == 6, "a letter's tail still counts");
		require(words.index_at({ -1, 10 }) == std::string::npos && words.index_at({ 18, 10 }) == std::string::npos,
			"left and right of a line hold nothing");
		require(words.index_at({ 1, 40 }) == std::string::npos, "nor does below the text");

		std::cout << "text layout tests passed\n";
		return 0;
	}
	catch (const std::exception &e)
	{
		std::cerr << e.what() << "\n";
		return 1;
	}
}
