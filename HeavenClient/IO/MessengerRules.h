#pragma once

#include <algorithm>
#include <cctype>
#include <cstddef>
#include <cstdint>
#include <string>
#include <vector>

// The parts of the v83 Maple Messenger (0x84dd83 on) that need no window:
// what a typed line means, what a heard line says, and when the room is told
// that we are typing.
namespace ms::messenger_rules
{
	inline bool same_name(const std::string &a, const std::string &b)
	{
		return a.size() == b.size() && std::equal(a.begin(), a.end(), b.begin(), [](unsigned char x, unsigned char y)
		{
			return std::tolower(x) == std::tolower(y);
		});
	}

	enum class Command
	{
		CHAT,
		QUIT,
		INVITE,
		// A slash line that means nothing: it goes nowhere.
		NONE
	};

	struct Typed
	{
		Command command;
		std::string name;
	};

	// A line starting with a slash is never said (0x851d3b). The whole line
	// "/q" leaves; "/invite " and anything after it, untrimmed, invites. Case
	// doesn't matter.
	inline Typed read(const std::string &text)
	{
		static const std::string QUIT = "/q";
		static const std::string INVITE = "/invite ";

		if (text.empty() || text[0] != '/')
			return { Command::CHAT, "" };

		if (same_name(text, QUIT))
			return { Command::QUIT, "" };

		if (text.size() > INVITE.size() && same_name(text.substr(0, INVITE.size()), INVITE))
			return { Command::INVITE, text.substr(INVITE.size()) };

		return { Command::NONE, "" };
	}

	struct Heard
	{
		std::string name;
		bool typing;
		// Said out loud, as opposed to a typing notice.
		bool chat;
	};

	// "name : text" is a chat line, which ends the speaker's typing. Anything
	// else is a typing notice: the name, then "0" if they stopped (0x8515d7).
	inline Heard hear(const std::string &line)
	{
		size_t at = line.find(" : ");

		if (at != std::string::npos)
			return { line.substr(0, at), false, true };

		if (line.empty())
			return { "", false, false };

		return { line.substr(0, line.size() - 1), line.back() != '0', false };
	}

	// The room hears when our input fills and when it empties, at most once
	// every 3 seconds (0x84e930). Until it has heard, ENTER stays grey.
	class TypingGate
	{
	public:
		enum class Notice
		{
			NONE,
			STARTED,
			STOPPED
		};

		Notice step(bool has_text, int32_t elapsed)
		{
			if (wait > 0)
			{
				wait -= elapsed;

				return Notice::NONE;
			}

			if (told == has_text)
				return Notice::NONE;

			told = has_text;
			wait = PAUSE;

			return told ? Notice::STARTED : Notice::STOPPED;
		}

		// A chat line says that we stopped.
		void sent()
		{
			told = false;
		}

		// Someone came in: at once, whatever the wait, they are told whether
		// we are typing (0x84f224).
		void arrival()
		{
			told = !told;
			wait = 0;
		}

	private:
		static constexpr int32_t PAUSE = 3000;

		bool told = false;
		int32_t wait = 0;
	};

	// The lines sent, walked with up and down as in the chat bar.
	class History
	{
	public:
		void push(const std::string &line)
		{
			lines.push_back(line);
			at = lines.size();
		}

		bool older(std::string &out)
		{
			if (at == 0)
				return false;

			out = lines[--at];

			return true;
		}

		bool newer(std::string &out)
		{
			if (lines.empty() || at + 1 >= lines.size())
				return false;

			out = lines[++at];

			return true;
		}

	private:
		std::vector<std::string> lines;
		size_t at = 0;
	};

	// The row under the log: "a,b is typing." (0x850ec9), or nothing.
	inline std::string typing_row(const std::vector<std::string> &names)
	{
		std::string row;

		for (const std::string &name : names)
		{
			if (!row.empty())
				row += ",";

			row += name;
		}

		return row.empty() ? row : row + " is typing.";
	}
}
