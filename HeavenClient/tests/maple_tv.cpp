#include "../Util/MapleTv.h"
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms::MapleTv;

		require(mode(5075000) == Mode::MESSENGER && mode(5075003) == Mode::MESSENGER, "kinds 0 and 3 are messengers");
		require(mode(5075001) == Mode::STAR && mode(5075004) == Mode::STAR, "kinds 1 and 4 are stars");
		require(mode(5075002) == Mode::HEART && mode(5075005) == Mode::HEART, "kinds 2 and 5 are hearts");
		require(!mega(5075002) && mega(5075003) && mega(5075005), "the last three are megassengers");

		require(receiver_flag("") == 1 && receiver_flag("Friend") == 3, "a receiver makes the flag 3");

		require(lines("", {}) == Lines{}, "nothing written is five empty lines");
		require(lines("hello world", { 0, 5 }) == Lines{ "hello ", "world", "", "", "" }, "a row a line");
		require(lines("abcdef", { 0, 1, 2, 3, 4, 5 }) == Lines{ "a", "b", "c", "d", "e" }, "a sixth row is left out");
		require(lines("a b", { 0, 1 }) == Lines{ "a ", "b", "", "", "" }, "the space wrapped at stays on the row before");
		require(lines("ab", { 0, 2 }) == Lines{ "ab", "", "", "", "" }, "a row starting past the end is empty");

		auto keep = [](const std::string& text) { return text; };
		auto star = [](const std::string& text) { return std::string(text.size(), '*'); };

		Lines empty;
		require(check(5075000, "", empty, keep) == Refusal::NONE, "an empty message is sent");
		require(check(5075002, "", empty, keep) == Refusal::NO_RECEIVER, "a heart needs a receiver");
		require(check(5075005, "", empty, keep) == Refusal::NO_RECEIVER, "a heart megassenger too");
		require(check(5075002, "Friend", empty, keep) == Refusal::NONE, "one named is enough");

		Lines bell{ "ring\a", "", "", "", "" };
		require(check(5075001, "", bell, keep) == Refusal::CURSING, "text the filter won't take is refused");

		Lines cut{ "ab", "cd", "", "", "" };
		require(check(5075001, "", cut, star) == Refusal::NONE, "the filter takes plain text");
		require(cut == Lines{ "**", "**", "", "", "" }, "censored as one text, cut back into the rows");

		auto bad = [](const std::string& text)
		{
			std::string censored = text;
			size_t at = censored.find("bad");

			if (at != std::string::npos)
				censored.replace(at, 3, "***");

			return censored;
		};
		Lines split{ "so b", "ad day", "", "", "" };
		require(check(5075001, "", split, bad) == Refusal::NONE, "a curse word is starred out");
		require(split == Lines{ "so *", "** day", "", "", "" }, "even cut by a wrap, in rows of their lengths");

		Lines rude{ "ring\a", "", "", "", "" };
		require(check(5075002, "", rude, keep) == Refusal::NO_RECEIVER, "the receiver is checked before the text");

		require(refusal_text(2) == "You've entered the wrong user name.", "code 2 is a wrong name");
		require(refusal_text(1).find("GM") != std::string::npos, "code 1 is for GMs only");
		require(refusal_text(3).find("an hour") != std::string::npos, "code 3 is a line over an hour");
		require(refusal_text(4).empty(), "nothing for another code");
		require(wait_question(75).find("in 75 seconds") != std::string::npos, "the question has the wait");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAIL: " << e.what() << std::endl;
		return 1;
	}

	return 0;
}
