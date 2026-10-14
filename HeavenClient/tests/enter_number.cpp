#include "../IO/UITypes/EnterNumber.h"
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
		using namespace ms::enter_number;

		require(check("25", 50).problem == Problem::NONE, "a number in range is accepted");
		require(check("25", 50).number == 25, "the accepted number is the one typed");
		require(check("50", 50).number == 50, "the maximum itself is accepted");
		require(check("007", 50).number == 7, "leading zeros are ignored");
		require(check("", 50).problem == Problem::TOO_LOW, "an empty box is too low");
		require(check("0", 50).problem == Problem::TOO_LOW, "zero is too low");
		require(check("51", 50).problem == Problem::TOO_HIGH, "one past the maximum is too high");
		require(check("9999999999", 50000).problem == Problem::TOO_HIGH, "ten digits are too high, not an overflow");
		require(check("2147483647", 2147483647).number == 2147483647, "the largest maximum is accepted");
		require(check("1a", 50).problem == Problem::NOT_A_NUMBER, "letters are not a number");
		require(check("-5", 50).problem == Problem::NOT_A_NUMBER, "a sign is not a number");

		std::cout << "Enter number passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
